//! Fixed deadline exits for owned neuro positions, independent of the AI worker.
use super::*;
pub async fn owned_quantity(db:&sqlx::PgPool,mode:ExecutionMode,parent:&str,base:&str)->Result<D>{
 let n:String=sqlx::query_scalar("SELECT COALESCE(sum(CASE WHEN side='buy' THEN quantity ELSE -quantity END + CASE WHEN fee_currency=$3 THEN fee ELSE 0 END),0)::text FROM execution.fills WHERE mode=$1 AND parent_id=$2").bind(mode.id()).bind(parent).bind(base).fetch_one(db).await?;Ok(n.parse::<D>()?.max(D::ZERO))
}
pub async fn deadlines(db:&sqlx::PgPool,mode:ExecutionMode)->Result<()> {
 // Parameters and timeframe come from the immutable artifact. A paused/revoked AI
 // cannot change the deadline or original stop of already acquired inventory.
 sqlx::query("UPDATE execution.orders o SET close_requested=true WHERE o.mode=$1 AND o.bot_release_id IS NOT NULL AND o.ticket->>'side'='buy' AND EXISTS(SELECT 1 FROM training.releases r JOIN training.artifacts a ON a.hash=r.artifact_hash WHERE r.id=o.bot_release_id AND $2 >= (SELECT min(fill_ms) FROM execution.fills f WHERE f.mode=o.mode AND f.parent_id=o.client_id AND f.side='buy') + (a.artifact->'parameters'->>'max_hold')::bigint * CASE r.bar WHEN '1m' THEN 60000 WHEN '5m' THEN 300000 WHEN '15m' THEN 900000 END)").bind(mode.id()).bind(now()).execute(db).await?;
 // Do not cancel protection while the parent can acquire more inventory.
 sqlx::query("UPDATE execution.orders SET command='CANCEL' WHERE mode=$1 AND close_requested=true AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).execute(db).await?;
 sqlx::query("UPDATE execution.protection p SET cancel_requested=true FROM execution.orders o WHERE p.mode=$1 AND o.mode=p.mode AND o.client_id=p.parent_id AND o.close_requested=true AND o.state IN ('FILLED','CANCELED','REJECTED') AND p.state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).execute(db).await?;Ok(())
}
pub async fn close_positions(db:&sqlx::PgPool,mode:ExecutionMode,s:&AccountSnapshot)->Result<()> {
 if !s.connected||!s.reconciled||!s.trade_permission||(now()-s.received_ms)>15000{return Ok(());}
 let parents=sqlx::query("SELECT client_id,ticket,bot_release_id,account_uid FROM execution.orders WHERE mode=$1 AND close_requested=true AND state IN ('FILLED','CANCELED','REJECTED') AND bot_release_id IS NOT NULL").bind(mode.id()).fetch_all(db).await?;
 for p in parents{
  let parent:String=p.get("client_id");let t:Ticket=serde_json::from_value(p.get("ticket"))?;let rid:String=p.get("bot_release_id");
  let Some(q)=s.quotes.get(&t.instrument) else{continue;};
  let remaining=owned_quantity(db,mode,&parent,&q.rules.base).await?;let quantity=(remaining/q.rules.lot).floor()*q.rules.lot;
  if quantity<q.rules.minimum{continue;}
  let mut tx=db.begin().await?;sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(if mode==ExecutionMode::OkxDemo{478007_i64}else{478008_i64}).execute(&mut *tx).await?;
  let uncertain:i64=sqlx::query_scalar("SELECT count(*) FROM execution.protection WHERE mode=$1 AND parent_id=$2 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).bind(&parent).fetch_one(&mut *tx).await?;if uncertain>0{continue;}
  let pending:i64=sqlx::query_scalar("SELECT count(*) FROM execution.orders WHERE mode=$1 AND reduction_parent_id=$2 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).bind(&parent).fetch_one(&mut *tx).await?;if pending>0{continue;}
  // One durable identity for this attempt. UNKNOWN attempts are never recreated.
  let seq:i64=sqlx::query_scalar("SELECT count(*) FROM execution.orders WHERE mode=$1 AND reduction_parent_id=$2").bind(mode.id()).bind(&parent).fetch_one(&mut *tx).await?;
  let last:i64=sqlx::query_scalar("SELECT COALESCE(max(created_ms),0) FROM execution.orders WHERE mode=$1 AND reduction_parent_id=$2").bind(mode.id()).bind(&parent).fetch_one(&mut *tx).await?;if now()-last<3000{continue;}
  let cid=format!("X{}{:X}",&parent[1..],seq);ensure!(cid.len()<=32,"Te veel exitpogingen; geen dubbele verzending");
  let price=((q.bid*(D::ONE-D::new(5,4)))/q.rules.tick).floor()*q.rules.tick;
  let ticket=Ticket{mode,instrument:t.instrument.clone(),side:Direction::Sell,quantity,order:OrderSpec::Limit{price,policy:LimitPolicy::Ioc},protection:None};
  // Native free balance plus all claims is rechecked again immediately at dispatch.
  let claims=sqlx::query("SELECT client_id,ticket,reserve::text,quantity::text,created_ms FROM execution.orders WHERE mode=$1 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).fetch_all(&mut *tx).await?.into_iter().map(|r|{let t:Ticket=serde_json::from_value(r.get("ticket"))?;Ok(Claim{id:r.get("client_id"),instrument:t.instrument,side:t.side,quantity:r.get::<String,_>("quantity").parse()?,reserve:r.get::<String,_>("reserve").parse()?,created_ms:r.get("created_ms"),active:true})}).collect::<Result<Vec<_>>>()?;
  let auth=order_risk::authorize_owned_reduction(&ticket,&p.get::<String,_>("account_uid"),s,&claims,remaining,now(),&cid)?;
  sqlx::query("INSERT INTO execution.orders(mode,client_id,account_uid,ticket,compiled,state,reserve_currency,reserve,quantity,created_ms,updated_ms,bot_release_id,reduction_parent_id) VALUES($1,$2,$3,$4,$5,'QUEUED',$6,$7::numeric,$8::numeric,$9,$9,$10,$11)").bind(mode.id()).bind(&cid).bind(&s.uid).bind(serde_json::to_value(ticket)?).bind(serde_json::to_value(auth.compiled())?).bind(auth.currency()).bind(auth.reserve().to_string()).bind(quantity.to_string()).bind(now()).bind(&rid).bind(&parent).execute(&mut *tx).await?;
  sqlx::query("INSERT INTO execution.events(mode,client_id,kind,detail,created_ms) VALUES($1,$2,'OWNED_DEADLINE_EXIT',$3,$4)").bind(mode.id()).bind(&cid).bind(json!({"parent":parent,"release":rid,"quantity":quantity.to_string()})).bind(now()).execute(&mut *tx).await?;tx.commit().await?;
 }
 Ok(())
}
