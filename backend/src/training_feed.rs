//! Public closed-candle stream with REST repair; sampled execution research tape.
use crate::{App,CandleQuery,candle_result,model::{Candle,now_ms},training};
use anyhow::{Result,ensure};
use futures_util::{SinkExt,StreamExt};
use serde_json::{Value,json};
use std::{collections::BTreeSet,time::Duration};
use tokio::{io::AsyncWriteExt,fs::OpenOptions};
use tokio_tungstenite::{connect_async,tungstenite::Message};
pub async fn run(a:App){loop{if let Err(e)=connection(&a).await{tracing::warn!(error=%e,"Research candle stream interrupted; REST repair on reconnect");}tokio::time::sleep(Duration::from_secs(5)).await;}}
async fn markets(a:&App)->Vec<String>{let mut set=BTreeSet::from(["BTC-EUR".to_string(),"ETH-EUR".to_string()]);if let Ok(rows)=sqlx::query_scalar::<_,String>("SELECT DISTINCT instrument FROM training.datasets").fetch_all(&a.db).await{set.extend(rows);}set.into_iter().take(32).collect()}
async fn store(a:&App,instrument:&str,c:&Candle)->Result<()> {ensure!(c.closed&&c.ts%60000==0&&c.ts+60000<=now_ms(),"Open/ongeldige onderzoekscandle");sqlx::query("INSERT INTO training.live_candles(instrument,ts,payload,received_ms) VALUES($1,$2,$3,$4) ON CONFLICT(instrument,ts) DO NOTHING").bind(instrument).bind(c.ts).bind(serde_json::to_value(c)?).bind(now_ms()).execute(&a.db).await?;Ok(())}
async fn repair(a:&App,instrument:&str)->Result<()> {
 let mut after=None;for _ in 0..11 {let data=candle_result(a,&CandleQuery{instrument:instrument.into(),bar:"1m".into(),after}).await?;let cs:Vec<Candle>=serde_json::from_value(data["candles"].clone())?;let Some(first)=cs.first()else{break};after=Some(first.ts);for c in cs.iter().filter(|c|c.closed&&c.ts+60000<=now_ms()){store(a,instrument,c).await?;}}
 Ok(())
}
async fn connection(a:&App)->Result<()> {
 let ids=markets(a).await;let (mut ws,_)=tokio::time::timeout(Duration::from_secs(15),connect_async("wss://wseea.okx.com/ws/v5/business")).await??;
 ws.send(Message::Text(json!({"op":"subscribe","args":ids.iter().map(|id|json!({"channel":"candle1m","instId":id})).collect::<Vec<_>>()} ).to_string().into())).await?;
 // Repair asynchronously; stream ingestion does not wait for historical pages.
 let copy=a.clone();let repair_ids=ids.clone();tokio::spawn(async move{for id in repair_ids{let _=repair(&copy,&id).await;}});
 let mut heart=tokio::time::interval(Duration::from_secs(15));let mut last=now_ms();
 loop {tokio::select!{
  _=heart.tick()=>{ensure!(now_ms()-last<45000,"Candle-WebSocket timeout");ensure!(markets(a).await==ids,"Onderzoeksmarkten veranderd; nieuwe subscription");ws.send(Message::Text("ping".into())).await?;},
  next=ws.next()=>{let Some(msg)=next else{break};let msg=msg?;if let Message::Text(text)=msg{last=now_ms();if text=="pong"{continue;}let v:Value=serde_json::from_str(&text)?;if v["event"]=="error"{anyhow::bail!("OKX candle-subscription geweigerd");}let id=v["arg"]["instId"].as_str().unwrap_or("");if !ids.iter().any(|s|s==id){continue;}if let Some(rows)=v["data"].as_array(){for row in rows{let c=Candle::parse(row)?;if c.closed&&c.ts+60000<=now_ms(){store(a,id,&c).await?;}}}}}
 }}Ok(())
}
pub async fn recent(a:&App,instrument:&str)->Result<Value> {
 let rows:Vec<Value>=sqlx::query_scalar("SELECT payload FROM (SELECT ts,payload FROM training.live_candles WHERE instrument=$1 ORDER BY ts DESC LIMIT 3300) c ORDER BY ts").bind(instrument).fetch_all(&a.db).await?;
 let cs:Vec<Candle>=serde_json::from_value(json!(rows))?;ensure!(!cs.is_empty(),"WebSocket warm-up ontbreekt");let gaps=cs.windows(2).filter(|w|w[1].ts-w[0].ts!=60000).count();let fresh=cs.last().is_some_and(|c|c.ts+75000>=now_ms());Ok(json!({"candles":cs,"gaps":gaps,"fresh":fresh,"received_ms":sqlx::query_scalar::<_,i64>("SELECT received_ms FROM training.live_candles WHERE instrument=$1 AND ts=$2").bind(instrument).bind(cs.last().unwrap().ts).fetch_one(&a.db).await?,"source":"OKX closed WebSocket candles; repaired by REST"}))
}
pub async fn record(a:App){
 let dir=training::root().join("market-tape");if tokio::fs::create_dir_all(&dir).await.is_err(){return;}
 let mut seen=std::collections::BTreeMap::new();let mut bytes=0u64;
 if let Ok(mut entries)=tokio::fs::read_dir(&dir).await{while let Ok(Some(p))=entries.next_entry().await{bytes+=p.metadata().await.map(|m|m.len()).unwrap_or(0);}}
 loop {
  if bytes<512*1024*1024{let ids=markets(&a).await;let m=a.market.read().await;let mut records=vec![];
   for id in ids {if let Some(book)=m.books.get(&id){let ts=book["received_at_ms"].as_i64().unwrap_or(0);let key=format!("book:{id}");if seen.get(&key)!=Some(&ts){seen.insert(key,ts);records.push(json!({"kind":"book_snapshot","instrument":id,"received_ms":now_ms(),"data":book,"scope":"2-second sampling; not a complete orderbook event stream"}));}}
    if let Some(trades)=m.trades.get(&id){let key=format!("trade:{id}");let prev=seen.get(&key).copied().unwrap_or(0);for trade in trades{let ts=trade["ts"].as_str().and_then(|s|s.parse::<i64>().ok()).unwrap_or(0);if ts>prev{records.push(json!({"kind":"trade","instrument":id,"received_ms":now_ms(),"data":trade}));seen.insert(key.clone(),ts);}}}
   }drop(m);
   if let Ok(mut file)=OpenOptions::new().create(true).append(true).open(dir.join(format!("{}.jsonl",now_ms()/86400000))).await{for r in records{if let Ok(mut data)=serde_json::to_vec(&r){data.push(b'\n');if file.write_all(&data).await.is_ok(){bytes+=data.len()as u64;}}}}
  }
  let status=json!({"recorded_bytes":bytes,"limit_bytes":512*1024*1024,"paused":bytes>=512*1024*1024,"scope":"sampled public books/trades; not sufficient for queue/maker replay"});let _=tokio::fs::write(dir.join("status.json"),status.to_string()).await;
  tokio::time::sleep(Duration::from_secs(2)).await;
 }
}
