//! Terminal proposals use the native account ledger; only the gateway has credentials.
use crate::{App,model::now_ms,neuro::{self,Artifact,Series},order_risk::{self,TradingLimits,AccountSnapshot},order_types::*,native_bot_risk,studio::reply};
use anyhow::{Result,ensure};
use axum::{Router,Json,extract::{State,Path},routing::{get,post},response::Response};
use rust_decimal::{Decimal as D,prelude::ToPrimitive};
use serde::Deserialize;
use serde_json::{Value,json};
use sqlx::Row;
use futures_util::StreamExt;
fn id()->String{let mut b=[0u8;12];use ring::rand::SecureRandom;ring::rand::SystemRandom::new().fill(&mut b).expect("OS random");b.iter().map(|b|format!("{b:02x}")).collect()}
fn dec(v:&Value)->Result<D>{Ok(v.as_str().ok_or_else(||anyhow::anyhow!("Decimaal ontbreekt"))?.parse()?)}
fn mode(v:&Value)->Result<ExecutionMode>{Ok(serde_json::from_value(v.clone())?)}
async fn audit(tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,kind:&str,subject:&str,detail:Value)->Result<()>{sqlx::query("INSERT INTO training.audit(kind,subject,detail,at_ms) VALUES($1,$2,$3,$4)").bind(kind).bind(subject).bind(detail).bind(now_ms()).execute(&mut **tx).await?;Ok(())}
pub fn router()->Router<App>{Router::new().route("/api/training/native",get(overview)).route("/api/training/artifacts/{hash}/native-release",post(release)).route("/api/training/native/{id}/{action}",post(control)).route("/api/training/native/{id}/accept-demo",post(accept_demo))}
async fn overview(State(a):State<App>)->Response{reply(async{
 let rows=sqlx::query("SELECT to_jsonb(r) AS release,n.status,n.report,n.updated_ms FROM training.releases r JOIN training.native_runs n ON n.release_id=r.id ORDER BY r.created_ms DESC LIMIT 100").fetch_all(&a.db).await?;
 let demo:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(d) FROM training.demo_acceptance d ORDER BY confirmed_ms DESC LIMIT 100").fetch_all(&a.db).await?;
 Ok(json!({"runs":rows.iter().map(|r|json!({"release":r.get::<Value,_>("release"),"status":r.get::<String,_>("status"),"report":r.get::<Value,_>("report"),"updated_ms":r.get::<i64,_>("updated_ms")})).collect::<Vec<_>>(),"demo":demo,"code_version":native_bot_risk::code_version()}))
 }.await)}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]struct Release {mode:ExecutionMode,limits:TradingLimits,confirmed:bool}
async fn release(State(a):State<App>,Path(hash):Path<String>,Json(r):Json<Release>)->Response{reply(async{
 ensure!(r.confirmed,"Bevestig deze exacte botversie, rekening, budget en limieten");r.limits.validate()?;
 let mut tx=a.db.begin().await?;crate::trading::lock(&mut tx,r.mode).await?;
 let artifact:Artifact=serde_json::from_value(sqlx::query_scalar::<_,Value>("SELECT artifact FROM training.artifacts WHERE hash=$1").bind(&hash).fetch_one(&mut *tx).await?)?;artifact.validate()?;
 let hist:Value=sqlx::query_scalar("SELECT report FROM training.holdouts WHERE artifact_hash=$1 AND status='PASSED' ORDER BY created_ms DESC LIMIT 1").bind(&hash).fetch_optional(&mut *tx).await?.ok_or_else(||anyhow::anyhow!("De exacte versie is historisch nog niet geslaagd"))?;
 let forward:Value=sqlx::query_scalar("SELECT report FROM training.forward_runs WHERE artifact_hash=$1 AND report->>'passed'='true' ORDER BY updated_ms DESC LIMIT 1").bind(&hash).fetch_optional(&mut *tx).await?.ok_or_else(||anyhow::anyhow!("Eerst werkelijk 30 dagen en 100 geslaagde forward-roundtrips"))?;
 ensure!(hist["family_complete"]==true&&hist["passed"]==true&&forward["passed"]==true,"Bewijsfamilie onvolledig");
 let ar=sqlx::query("SELECT snapshot,policy,policy_revision FROM execution.accounts WHERE mode=$1 FOR UPDATE").bind(r.mode.id()).fetch_one(&mut *tx).await?;
 let s:AccountSnapshot=serde_json::from_value(ar.get::<Option<Value>,_>("snapshot").ok_or_else(||anyhow::anyhow!("Verbind eerst deze OKX-rekening"))?)?;
 let account:TradingLimits=serde_json::from_value(ar.get::<Option<Value>,_>("policy").ok_or_else(||anyhow::anyhow!("Bevestig eerst je rekeninglimieten"))?)?;
 native_bot_risk::limits_within(&r.limits,&account,&artifact.instrument)?;
 ensure!(s.uid==r.limits.account_uid&&s.mode==r.mode&&s.connected&&s.reconciled&&s.trade_permission&&s.native_protection_verified&&(0..=15000).contains(&(now_ms()-s.received_ms)),"Private account, bescherming en reconciliatie moeten actueel zijn");
 let fee=order_risk::account_fee(&s,&artifact.instrument)?;let q=s.quotes.get(&artifact.instrument).ok_or_else(||anyhow::anyhow!("Markt ontbreekt"))?;
 let fx=order_risk::euro_bounds(&s,&q.rules.quote,now_ms())?.1;
 let historical_cost:Value=sqlx::query_scalar("SELECT c.config->'costs' FROM training.artifacts a JOIN training.campaigns c ON c.id=a.campaign_id WHERE a.hash=$1").bind(&hash).fetch_one(&mut *tx).await?;
 let c:neuro::Costs=serde_json::from_value(historical_cost)?;
 let historical=(c.taker_fee.to_f64().unwrap_or(0.)*2.+(c.spread_bps+c.slippage_bps*2.+c.impact_bps*2.)/10000.)*1.5;
 let current=fee.to_f64().unwrap_or(0.)*2.+((q.ask-q.bid)/q.bid).to_f64().unwrap_or(f64::INFINITY)+0.001;
 ensure!(current<=historical,"Actuele fees, spread en IOC-prijsgrens vallen buiten de bewezen kostenstress; nieuwe kwalificatie nodig");
 let code=native_bot_risk::code_version();
 let gateway:Option<Value>=sqlx::query_scalar("SELECT detail FROM execution.events WHERE mode=$1 AND kind='GATEWAY_STARTED' ORDER BY id DESC LIMIT 1").bind(r.mode.id()).fetch_optional(&mut *tx).await?;
 ensure!(gateway.as_ref().is_some_and(|g|g["code_version"]==code),"Start eerst de bijgewerkte ordergateway; de uitvoeringscode moet exact overeenkomen");
 let demo=if r.mode==ExecutionMode::Live{let d:Value=sqlx::query_scalar("SELECT to_jsonb(d) FROM training.demo_acceptance d WHERE artifact_hash=$1 AND code_version=$2 ORDER BY confirmed_ms DESC LIMIT 1").bind(&hash).bind(&code).fetch_optional(&mut *tx).await?.ok_or_else(||anyhow::anyhow!("Echte Demo-acceptatie ontbreekt voor deze exacte versie en uitvoeringscode"))?;json!({"accepted":true,"evidence":d})}else{json!({"accepted":false})};
 let active_count:i64=sqlx::query_scalar("SELECT count(*) FROM training.releases WHERE mode=$1 AND status='ACTIVE'").bind(r.mode.id()).fetch_one(&mut *tx).await?;
 ensure!(active_count<8,"Maximaal acht actieve native botversies per rekening binnen het lokale uitvoeringsprofiel");
 let total:String=sqlx::query_scalar("SELECT COALESCE(sum((limits->>'capital')::numeric),0)::text FROM training.releases WHERE mode=$1 AND status='ACTIVE'").bind(r.mode.id()).fetch_one(&mut *tx).await?;
 ensure!(total.parse::<D>()?+r.limits.capital<=account.capital,"Gezamenlijke botbudgetten overschrijden het rekeningbudget");
 let duplicate:i64=sqlx::query_scalar("SELECT count(*) FROM training.releases WHERE mode=$1 AND artifact_hash=$2 AND status='ACTIVE'").bind(r.mode.id()).bind(&hash).fetch_one(&mut *tx).await?;ensure!(duplicate==0,"Deze versie is al vrijgegeven in deze omgeving");
 let rid=id();let cash=r.limits.capital/fx;
 let qualification=json!({"historical":hist,"forward":forward,"demo":demo,"account_fee":fee.to_string(),"execution_rate_ceiling":historical,"confirmed":true});
 sqlx::query("INSERT INTO training.releases(id,artifact_hash,account_uid,mode,instrument,bar,limits,qualification,status,created_ms,policy_revision,code_version) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'ACTIVE',$9,$10,$11)").bind(&rid).bind(&hash).bind(&s.uid).bind(r.mode.id()).bind(&artifact.instrument).bind(&artifact.bar).bind(serde_json::to_value(&r.limits)?).bind(qualification).bind(now_ms()).bind(ar.get::<i64,_>("policy_revision")).bind(code).execute(&mut *tx).await?;
 sqlx::query("INSERT INTO training.native_runs(release_id,status,state,updated_ms) VALUES($1,'RUNNING',$2,$3)").bind(&rid).bind(json!({"initial_cash":cash.to_string(),"quote":q.rules.quote,"start_equity":r.limits.capital.to_string(),"high":r.limits.capital.to_string(),"day_start":r.limits.capital.to_string(),"day":chrono::Utc::now().format("%Y-%m-%d").to_string()})).bind(now_ms()).execute(&mut *tx).await?;
 audit(&mut tx,"EXACT_BOT_RELEASE",&rid,json!({"artifact_hash":hash,"mode":r.mode,"limits":r.limits})).await?;tx.commit().await?;Ok(json!({"id":rid,"status":"RUNNING","mode":r.mode}))
 }.await)}
async fn control(State(a):State<App>,Path((rid,action)):Path<(String,String)>)->Response{reply(async{
 ensure!(["pause","resume","revoke"].contains(&action.as_str()),"Kies pauze, hervat of intrekken");
 let m:Value=sqlx::query_scalar("SELECT to_jsonb(r) FROM training.releases r WHERE id=$1").bind(&rid).fetch_one(&a.db).await?;let mde=mode(&m["mode"])?;
 let mut tx=a.db.begin().await?;crate::trading::lock(&mut tx,mde).await?;
 let status:String=sqlx::query_scalar("SELECT status FROM training.native_runs WHERE release_id=$1 FOR UPDATE").bind(&rid).fetch_one(&mut *tx).await?;
 ensure!(m["status"]=="ACTIVE","Deze vrijgave is ingetrokken; een nieuwe vrijgave is vereist");
 if action=="resume"{ensure!(status!="CODE_CHANGED","Uitvoeringscode gewijzigd; nieuwe Demo-acceptatie en vrijgave vereist");let snapshot:Option<Value>=sqlx::query_scalar("SELECT snapshot FROM execution.accounts WHERE mode=$1").bind(mde.id()).fetch_one(&mut *tx).await?;let s:AccountSnapshot=serde_json::from_value(snapshot.ok_or_else(||anyhow::anyhow!("Rekening ontbreekt"))?)?;ensure!(s.connected&&s.reconciled&&s.native_protection_verified&&(0..=15000).contains(&(now_ms()-s.received_ms)),"Herstel eerst de actuele rekening en bescherming");}
 let next=if action=="resume"{"RUNNING"}else if action=="pause"{"PAUSED"}else{"REVOKED"};
 sqlx::query("UPDATE training.native_runs SET status=$2,updated_ms=$3 WHERE release_id=$1").bind(&rid).bind(next).bind(now_ms()).execute(&mut *tx).await?;
 if action=="revoke"{sqlx::query("UPDATE training.releases SET status='REVOKED' WHERE id=$1").bind(&rid).execute(&mut *tx).await?;}
 // Protection is owned by the gateway and remains active independently of bot status.
 sqlx::query("UPDATE execution.orders SET state='REJECTED',reserve=0,reason='Botentries gepauzeerd of ingetrokken' WHERE bot_release_id=$1 AND state='QUEUED' AND ticket->>'side'='buy'").bind(&rid).execute(&mut *tx).await?;
 audit(&mut tx,"BOT_CONTROL",&rid,json!({"status":next,"previous":status})).await?;tx.commit().await?;Ok(json!({"id":rid,"status":next}))
 }.await)}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]struct Confirm {confirmed:bool}
async fn accept_demo(State(a):State<App>,Path(rid):Path<String>,Json(r):Json<Confirm>)->Response{reply(async{
 ensure!(r.confirmed,"Bevestig het gecontroleerde Demo-bewijs");let mut tx=a.db.begin().await?;crate::trading::lock(&mut tx,ExecutionMode::OkxDemo).await?;
 let release:Value=sqlx::query_scalar("SELECT to_jsonb(r) FROM training.releases r WHERE id=$1 FOR UPDATE").bind(&rid).fetch_one(&mut *tx).await?;
 ensure!(release["mode"]=="OKX_DEMO"&&release["code_version"]==native_bot_risk::code_version(),"Demo-versie en uitvoeringscode moeten overeenkomen");
 let entries:i64=sqlx::query_scalar("SELECT count(*) FROM execution.orders WHERE bot_release_id=$1 AND ticket->>'side'='buy' AND filled>0 AND state IN ('FILLED','CANCELED')").bind(&rid).fetch_one(&mut *tx).await?;
 let partials:i64=sqlx::query_scalar("SELECT count(*) FROM execution.orders o WHERE o.bot_release_id=$1 AND o.ticket->>'side'='buy' AND EXISTS(SELECT 1 FROM execution.fills f WHERE f.mode=o.mode AND f.parent_id=o.client_id AND f.side='buy' AND f.quantity<o.quantity)").bind(&rid).fetch_one(&mut *tx).await?;
 let protections:i64=sqlx::query_scalar("SELECT count(*) FROM execution.protection p JOIN execution.orders o ON o.mode=p.mode AND o.client_id=p.parent_id WHERE o.bot_release_id=$1 AND p.state IN ('ACTIVE','TRIGGERED','FILLED','CANCELED') AND p.exchange_id IS NOT NULL").bind(&rid).fetch_one(&mut *tx).await?;
 let exits:i64=sqlx::query_scalar("SELECT count(*) FROM execution.fills f JOIN execution.orders o ON o.mode=f.mode AND o.client_id=f.parent_id WHERE o.bot_release_id=$1 AND f.side='sell'").bind(&rid).fetch_one(&mut *tx).await?;
 let recovered:i64=sqlx::query_scalar("SELECT count(*) FROM execution.events e JOIN execution.orders o ON o.mode=e.mode AND o.client_id=e.client_id WHERE o.bot_release_id=$1 AND e.kind='RECOVERED_UNKNOWN'").bind(&rid).fetch_one(&mut *tx).await?;
 let starts:i64=sqlx::query_scalar("SELECT count(*) FROM execution.events WHERE mode='OKX_DEMO' AND kind='GATEWAY_STARTED' AND created_ms>=$1").bind(release["created_ms"].as_i64().unwrap_or(i64::MAX)).fetch_one(&mut *tx).await?;
 let unresolved:i64=sqlx::query_scalar("SELECT count(*) FROM execution.orders WHERE mode='OKX_DEMO' AND state IN ('UNKNOWN','SUBMITTING')").fetch_one(&mut *tx).await?;
 let sv:Option<Value>=sqlx::query_scalar("SELECT snapshot FROM execution.accounts WHERE mode='OKX_DEMO'").fetch_one(&mut *tx).await?;let s:AccountSnapshot=serde_json::from_value(sv.ok_or_else(||anyhow::anyhow!("Demo-account ontbreekt"))?)?;
 ensure!(entries>0&&partials>0&&protections>0&&exits>0&&recovered>0&&starts>=2&&unresolved==0&&s.uid==release["account_uid"]&&s.connected&&s.reconciled&&s.native_protection_verified&&(0..=15000).contains(&(now_ms()-s.received_ms)),"Demo-bewijs onvolledig: echte entry, partial fill, native bescherming, exit, herstart en opgelost onzeker orderverloop zijn vereist");
 let evidence=json!({"entries":entries,"partial_fills":partials,"protections":protections,"exits":exits,"recovered_unknown":recovered,"gateway_starts":starts,"unresolved":unresolved,"current_reconciled":true,"source":"actual OKX_DEMO gateway ledger"});
 let did=id();sqlx::query("INSERT INTO training.demo_acceptance(id,release_id,artifact_hash,code_version,evidence,confirmed_ms) VALUES($1,$2,$3,$4,$5,$6)").bind(&did).bind(&rid).bind(release["artifact_hash"].as_str().unwrap()).bind(native_bot_risk::code_version()).bind(&evidence).bind(now_ms()).execute(&mut *tx).await?;audit(&mut tx,"DEMO_ACCEPTED",&rid,evidence).await?;tx.commit().await?;Ok(json!({"accepted":true,"id":did}))
 }.await)}
async fn ledger(a:&App,rid:&str,s:&AccountSnapshot,state:&Value)->Result<(D,Value,Vec<Value>)>{
 let fills:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(f)||jsonb_build_object('quantity',f.quantity::text,'price',f.price::text,'fee',f.fee::text) FROM execution.fills f JOIN execution.orders o ON o.mode=f.mode AND o.client_id=f.parent_id WHERE o.bot_release_id=$1 ORDER BY f.fill_ms,f.bill_id LIMIT 100001").bind(rid).fetch_all(&a.db).await?;
 ensure!(fills.len()<=100000,"Botadministratie vereist gepagineerde archivering; geen stille afkap");
 let quote=state["quote"].as_str().ok_or_else(||anyhow::anyhow!("Botcash-valuta ontbreekt"))?;let mut holdings:std::collections::BTreeMap<String,D>=std::collections::BTreeMap::from([(quote.into(),dec(&state["initial_cash"])?) ]);
 for f in &fills{let inst=f["instrument"].as_str().ok_or_else(||anyhow::anyhow!("Fillmarkt ontbreekt"))?;let (base,q)=inst.split_once('-').ok_or_else(||anyhow::anyhow!("Fillmarkt ongeldig"))?;let qty=dec(&f["quantity"])?;let price=dec(&f["price"])?;let side=if f["side"]=="buy"{D::ONE}else{-D::ONE};*holdings.entry(base.into()).or_default()+=qty*side;*holdings.entry(q.into()).or_default()-=qty*price*side;*holdings.entry(f["fee_currency"].as_str().ok_or_else(||anyhow::anyhow!("Feevaluta ontbreekt"))?.into()).or_default()+=dec(&f["fee"])?;}
 let mut equity=D::ZERO;for(ccy,qty)in &holdings{if *qty!=D::ZERO{let (bid,ask)=order_risk::euro_bounds(s,ccy,now_ms())?;equity+=*qty*if *qty>=D::ZERO{bid}else{ask};if *qty>D::ZERO&&ccy!=quote {let fee=s.fee_rate.max(D::ZERO);equity-=*qty*bid*(fee+D::new(5,4));}}}
 Ok((equity,serde_json::to_value(holdings)?,fills))
}
pub async fn run(a:App){
 let Ok(mut owner)=a.db.acquire().await else{return;};let Ok(true)=sqlx::query_scalar::<_,bool>("SELECT pg_try_advisory_lock(775904)").fetch_one(&mut *owner).await else{return;};
 loop{if sqlx::query("SELECT 1").execute(&mut *owner).await.is_err(){break;}
  if let Ok(ids)=sqlx::query_scalar::<_,String>("SELECT release_id FROM training.native_runs ORDER BY updated_ms DESC LIMIT 100").fetch_all(&a.db).await{
    futures_util::stream::iter(ids).map(|rid|{let a=&a;async move{if let Err(e)=tick(a,&rid).await{let _=sqlx::query("UPDATE training.native_runs SET report=report||$2,updated_ms=$3 WHERE release_id=$1").bind(&rid).bind(json!({"reason":e.to_string(),"entries_blocked":true})).bind(now_ms()).execute(&a.db).await;}}}).buffer_unordered(2).collect::<Vec<_>>().await;
  }
  tokio::time::sleep(std::time::Duration::from_millis(350)).await;
 }
}
async fn tick(a:&App,rid:&str)->Result<()> {
 let row=sqlx::query("SELECT to_jsonb(r) AS release,n.state,n.status,n.last_ms,a.artifact FROM training.native_runs n JOIN training.releases r ON r.id=n.release_id JOIN training.artifacts a ON a.hash=r.artifact_hash WHERE n.release_id=$1").bind(rid).fetch_one(&a.db).await?;
 let release:Value=row.get("release");let m=mode(&release["mode"])?;let art:Artifact=serde_json::from_value(row.get("artifact"))?;art.validate()?;let mut state:Value=row.get("state");let status:String=row.get("status");let last:i64=row.get("last_ms");
 sqlx::query("INSERT INTO execution.market_interest(mode,instrument,updated_ms) VALUES($1,$2,$3) ON CONFLICT(mode,instrument) DO UPDATE SET updated_ms=$3").bind(m.id()).bind(&art.instrument).bind(now_ms()).execute(&a.db).await?;
 let sv:Option<Value>=sqlx::query_scalar("SELECT snapshot FROM execution.accounts WHERE mode=$1").bind(m.id()).fetch_one(&a.db).await?;let s:AccountSnapshot=serde_json::from_value(sv.ok_or_else(||anyhow::anyhow!("Gatewayrekening ontbreekt"))?)?;
 ensure!(s.uid==release["account_uid"]&&s.mode==m&&s.connected&&s.reconciled&&(0..=15000).contains(&(now_ms()-s.received_ms)),"Private rekening niet actueel of reconciliatie ontbreekt");
 let (equity,holdings,fills)=ledger(a,rid,&s,&state).await?;let limits:TradingLimits=serde_json::from_value(release["limits"].clone())?;
 let high=dec(&state["high"])?.max(equity);let day=chrono::Utc::now().format("%Y-%m-%d").to_string();if state["day"]!=day{state["day_start"]=json!(equity.to_string());state["day"]=json!(day);}state["high"]=json!(high.to_string());
 let loss=(dec(&state["start_equity"])?-equity).max(D::ZERO);let dd=(high-equity).max(D::ZERO);let day_loss=(dec(&state["day_start"])?-equity).max(D::ZERO);
 let risk=loss>=limits.total_loss||dd>=limits.drawdown||day_loss>=limits.daily_loss;let code_changed=release["code_version"]!=native_bot_risk::code_version();let runstatus=if code_changed{"CODE_CHANGED"}else if risk||status=="RISK_STOP"{"RISK_STOP"}else{status.as_str()};
 let entries=runstatus=="RUNNING"&&release["status"]=="ACTIVE";
 let mut report=json!({"equity_eur":equity.to_string(),"net_pnl_eur":(equity-dec(&state["start_equity"])?).to_string(),"drawdown_eur":dd.to_string(),"daily_loss_eur":day_loss.to_string(),"total_loss_eur":loss.to_string(),"holdings":holdings,"fills":fills.len(),"entries_blocked":!entries,"reason":if code_changed{"Uitvoeringscode gewijzigd; nieuwe Demo-acceptatie en vrijgave"}else if risk{"Vaste botverlieslimiet bereikt; entries gestopt, bescherming blijft actief"}else{"Native bescherming blijft onafhankelijk bij de gateway"}});
 sqlx::query("UPDATE training.native_runs SET state=$2,status=$3,report=report||$4,updated_ms=$5 WHERE release_id=$1 AND status=$6").bind(rid).bind(&state).bind(runstatus).bind(&report).bind(now_ms()).bind(&status).execute(&a.db).await?;
 let latest:Option<i64>=sqlx::query_scalar("SELECT max(ts) FROM training.live_candles WHERE instrument=$1").bind(&art.instrument).fetch_one(&a.db).await?;
 if latest.is_some_and(|ts|ts < last+crate::model::interval(&art.bar).unwrap()){return Ok(());}
 // The gateway closes expired owned positions independently of Laya or feed freshness.
 let data=crate::training_feed::recent(a,&art.instrument).await?;ensure!(data["fresh"]==true&&data["gaps"]==0,"Gesloten candles ontbreken of zijn verouderd");let cs:Vec<crate::model::Candle>=serde_json::from_value(data["candles"].clone())?;
 let step=crate::model::interval(&art.bar).unwrap();let bars=neuro::aggregate(&cs,step)?;ensure!(bars.len()>neuro::WARMUP,"Wacht op voldoende gesloten candles voor warm-up");let mut series=Series::new(bars,step)?;let i=series.candles.len()-1;let ts=series.candles[i].ts;
 if ts<=last{return Ok(());}ensure!(ts+step<=now_ms()&&now_ms()-ts-step<=15000,"Geen verse gesloten candle");
 if art.laya{let vals=crate::training::laya_values(a,&series.raw[i],ts,&art.instrument,&art.bar,&art.laya_runtime).await?;series.laya.insert(ts,vals);}
 let base=art.instrument.split('-').next().unwrap();let total_qty=holdings[base].as_str().and_then(|n|n.parse::<D>().ok()).unwrap_or_default();
 let rules=&s.quotes.get(&art.instrument).ok_or_else(||anyhow::anyhow!("Botmarkt ontbreekt"))?.rules;
 let qty=if total_qty>=rules.minimum{total_qty}else{D::ZERO};let last_buy=fills.iter().rev().find(|f|f["side"]=="buy");let last_sell=fills.iter().rev().find(|f|f["side"]=="sell");
 let q=s.quotes.get(&art.instrument).ok_or_else(||anyhow::anyhow!("Actuele markt ontbreekt"))?;
 let cost_rate=order_risk::account_fee(&s,&art.instrument)?.to_f64().unwrap_or(0.)*2.+((q.ask-q.bid)/q.bid).to_f64().unwrap_or(0.)+0.001;
 let owned_buys:Vec<_>=fills.iter().filter(|f|f["side"]=="buy"&&last_buy.is_some_and(|b|f["parent_id"]==b["parent_id"])).collect();
 let mut sum_qty=D::ZERO;let mut sum_value=D::ZERO;for b in &owned_buys{let q=dec(&b["quantity"])?;sum_qty+=q;sum_value+=q*dec(&b["price"])?;}
 let entry=if qty>D::ZERO&&sum_qty>D::ZERO{Some(sum_value/sum_qty)}else{None};
 let opened=owned_buys.iter().filter_map(|b|b["fill_ms"].as_i64()).min().unwrap_or(ts);
 let age=((ts/step-opened/step+1).max(0))as usize;
 let raw=neuro::decision_inputs(&series,i,entry,age,art.parameters.max_hold,cost_rate,art.laya);let act=neuro::activate(&art,raw);let signal=neuro::entry_signal(&series,i,&art.parameters,act.selected,&raw);
 let cooldown=last_sell.is_some_and(|f|now_ms()-f["fill_ms"].as_i64().unwrap_or(now_ms())<step*art.parameters.cooldown as i64);
 let mut tx=a.db.begin().await?;crate::trading::lock(&mut tx,m).await?;
 let current=sqlx::query("SELECT n.last_ms,n.status,to_jsonb(r) AS release FROM training.native_runs n JOIN training.releases r ON r.id=n.release_id WHERE n.release_id=$1 FOR UPDATE OF n,r").bind(rid).fetch_one(&mut *tx).await?;
 if current.get::<i64,_>("last_ms")>=ts{return Ok(());}let active=current.get::<String,_>("status")=="RUNNING"&&current.get::<Value,_>("release")["status"]=="ACTIVE";
 let pending:i64=sqlx::query_scalar("SELECT count(*) FROM execution.orders WHERE bot_release_id=$1 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(rid).fetch_one(&mut *tx).await?;
 let reason=if !active{"Entries gepauzeerd"}else if qty>D::ZERO{"Eigen positie behoudt oorspronkelijke exits"}else if pending>0{"Orderuitkomst wordt eerst gereconcilieerd"}else if cooldown{"Cooldown"}else if !signal{"Strategiefilter geeft geen instapsignaal"}else{"Gesloten candle en strategiefilter geslaagd"};
 report["activation"]=serde_json::to_value(&act)?;report["last_decision"]=json!({"ts":ts,"strategy":neuro::STRATEGIES[act.selected],"action":"wait","reason":reason});
 if active&&entries&&signal&&qty<=D::ZERO&&pending==0&&!cooldown{
  let g:Option<Value>=sqlx::query_scalar("SELECT detail FROM execution.events WHERE mode=$1 AND kind='GATEWAY_STARTED' ORDER BY id DESC LIMIT 1").bind(m.id()).fetch_optional(&mut *tx).await?;
  ensure!(g.as_ref().is_some_and(|g|g["code_version"]==native_bot_risk::code_version()),"Gatewaycode wijkt af; entries wachten op dezelfde gecontroleerde versie");
  let ar=sqlx::query("SELECT snapshot,policy,policy_revision FROM execution.accounts WHERE mode=$1 FOR UPDATE").bind(m.id()).fetch_one(&mut *tx).await?;
  let mut snap:AccountSnapshot=serde_json::from_value(ar.get::<Value,_>("snapshot"))?;let policy:TradingLimits=serde_json::from_value(ar.get::<Value,_>("policy"))?;
  let quotes=crate::paper::native_quotes(a).await;let pubq=quotes.get(&art.instrument).ok_or_else(||anyhow::anyhow!("Verse marktprijs ontbreekt"))?;let q=snap.quotes.get_mut(&art.instrument).ok_or_else(||anyhow::anyhow!("Accountmarkt ontbreekt"))?;ensure!(q.rules.tick==pubq.tick&&q.rules.lot==pubq.lot&&q.rules.minimum==pubq.minimum,"Instrumentregels gewijzigd");q.bid=pubq.bid;q.ask=pubq.ask;q.received_ms=pubq.received_ms;
  let q=q.clone();let fx=order_risk::euro_bounds(&snap,&q.rules.quote,now_ms())?.1;let fee=order_risk::account_fee(&snap,&art.instrument)?;
  let price=((q.ask*(D::ONE+D::new(5,4)))/q.rules.tick).ceil()*q.rules.tick;
  let amount=(limits.capital*D::new(1,1)).min(limits.max_order)/fx;let quantity=((amount/(price*(D::ONE+fee)))/q.rules.lot).floor()*q.rules.lot;
  let stop=((price*(D::ONE-D::from_f64_retain(art.parameters.stop_bps/10000.).unwrap()))/q.rules.tick).ceil()*q.rules.tick;
  let target=((price*(D::ONE+D::from_f64_retain(art.parameters.target_bps/10000.).unwrap()))/q.rules.tick).floor()*q.rules.tick;
  let ticket=Ticket{mode:m,instrument:art.instrument.clone(),side:Direction::Buy,quantity,order:OrderSpec::Limit{price,policy:LimitPolicy::Ioc},protection:Some(Protection{stop,target:Some(target)})};let cid=format!("N{}",id());
  native_bot_risk::check_binding(&release,&ticket,ar.get("policy_revision"),&policy,true,false)?;native_bot_risk::check_cost_envelope(&release,&snap)?;snap.live_enabled=m==ExecutionMode::Live;let claims=crate::trading::claims(&mut tx,m).await?;
  let auth=order_risk::authorize(&ticket,&policy,&snap,&claims,now_ms(),&cid)?;let owned_ids:Vec<String>=sqlx::query_scalar("SELECT client_id FROM execution.orders WHERE bot_release_id=$1 UNION SELECT p.client_id FROM execution.protection p JOIN execution.orders o ON o.mode=p.mode AND o.client_id=p.parent_id WHERE o.bot_release_id=$1").bind(rid).fetch_all(&mut *tx).await?;
  let owned_claims:Vec<_>=claims.iter().filter(|c|owned_ids.contains(&c.id)).cloned().collect();
  let own=order_risk::scoped_bot_snapshot(&snap,&report)?;order_risk::authorize(&ticket,&limits,&own,&owned_claims,now_ms(),&cid)?;
  sqlx::query("INSERT INTO execution.orders(mode,client_id,account_uid,ticket,compiled,state,reserve_currency,reserve,quantity,created_ms,updated_ms,bot_release_id) VALUES($1,$2,$3,$4,$5,'QUEUED',$6,$7::numeric,$8::numeric,$9,$9,$10)").bind(m.id()).bind(&cid).bind(&snap.uid).bind(serde_json::to_value(ticket)?).bind(serde_json::to_value(auth.compiled())?).bind(auth.currency()).bind(auth.reserve().to_string()).bind(quantity.to_string()).bind(now_ms()).bind(rid).execute(&mut *tx).await?;
  report["proposal_prepare_ms"]=json!(now_ms()-data["received_ms"].as_i64().unwrap_or(now_ms()));
  report["last_decision"]["action"]=json!("propose_buy");report["last_decision"]["client_id"]=json!(cid);
 }
 sqlx::query("INSERT INTO training.native_decisions(release_id,candle_ms,kind,client_id,detail,created_ms) VALUES($1,$2,'CANDLE',$3,$4,$5)").bind(rid).bind(ts).bind(report["last_decision"]["client_id"].as_str()).bind(&report).bind(now_ms()).execute(&mut *tx).await?;
 let proposed=report["last_decision"]["action"]=="propose_buy";
 sqlx::query("UPDATE training.native_runs SET last_ms=$2,report=report||$3,updated_ms=$4 WHERE release_id=$1").bind(rid).bind(ts).bind(report).bind(now_ms()).execute(&mut *tx).await?;tx.commit().await?;
 if proposed{let latency=now_ms()-data["received_ms"].as_i64().unwrap_or(now_ms());sqlx::query("UPDATE training.native_runs SET report=report||$3 WHERE release_id=$1 AND last_ms=$2").bind(rid).bind(ts).bind(json!({"proposal_latency_ms":latency,"proposal_latency_scope":"closed-candle first receipt to durable queue transaction commit; excludes exchange ACK"})).execute(&a.db).await?;}
 Ok(())
}
