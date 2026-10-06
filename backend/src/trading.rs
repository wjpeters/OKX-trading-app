//! Credential-free control plane. Only the separate gateway can contact private OKX.
use crate::{
    App,
    model::now_ms,
    order_risk::{self, AccountSnapshot, Claim, TradingLimits},
    order_types::*,
    studio::reply,
};
use anyhow::{Result, ensure};
use axum::{
    Json, Router,
    extract::{Query, State},
    response::Response,
    routing::{get, post},
};
use rust_decimal::Decimal as D;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};

pub fn router() -> Router<App> {
    Router::new()
        .route("/api/trading", get(status))
        .route("/api/trading/catalog", get(catalogue))
        .route("/api/trading/balances", get(balances))
        .route("/api/trading/size", post(size))
        .route("/api/trading/live", post(live_release))
        .route("/api/trading/limits", post(limits))
        .route("/api/trading/preview", post(preview))
        .route("/api/trading/orders", get(orders))
        .route("/api/trading/activity", get(activity))
        .route("/api/trading/submit", post(submit))
        .route("/api/trading/cancel", post(cancel))
}
#[derive(Deserialize)]
pub struct ModeQuery {
    pub mode: ExecutionMode,
    pub instrument: Option<String>,
}
async fn catalogue() -> Json<Value> {
    Json(catalog())
}
fn apply_release(
    s: &mut AccountSnapshot,
    p: &TradingLimits,
    row: &sqlx::postgres::PgRow,
    now: i64,
) -> Result<()> {
    let release = row
        .get::<Option<Value>, _>("live_release")
        .map(serde_json::from_value::<order_risk::LiveRelease>)
        .transpose()?;
    order_risk::apply_live_release(
        s,
        Some(p),
        release.as_ref(),
        row.get("policy_revision"),
        now,
    );
    Ok(())
}
async fn balances(State(a): State<App>, Query(q): Query<ModeQuery>) -> Response {
    reply(async {
        let row=sqlx::query("SELECT snapshot,status,reason,updated_ms FROM execution.wallets WHERE mode=$1").bind(q.mode.id()).fetch_one(&a.db).await?;
        let configured=crate::secrets::configured(if q.mode==ExecutionMode::Live{"okx_live"}else{"okx_demo"}).await;
        let snapshot=row.get::<Option<Value>,_>("snapshot");
        let valid_time=snapshot.as_ref().and_then(|v|v["trading_received_ms"].as_i64()).is_some_and(|t|(0..=15000).contains(&(now_ms()-t)));
        let state:String=row.get("status");
        let fresh=configured && valid_time && state=="BALANCES_CONNECTED" && (0..=15000).contains(&(now_ms()-row.get::<i64,_>("updated_ms")));
        let reason=if !configured{"Voeg je OKX-aansluiting toe onder Systeem → Aansluitingen & AI.".to_string()}else if row.get::<i64,_>("updated_ms")==0{"Rekeningverbinding wordt gestart…".into()}else if !valid_time && state=="BALANCES_CONNECTED"{"Saldo verouderd; wacht op de rekeningverbinding.".into()}else{row.get("reason")};
        Ok(json!({"mode":q.mode,"fresh":fresh,"configured":configured,"status":state,"reason":reason,"snapshot":snapshot}))
    }.await)
}
async fn size(State(a): State<App>, Json(r): Json<crate::order_wallet::SizeRequest>) -> Response {
    reply(
        async {
            let mut tx = a.db.begin().await?;
            lock(&mut tx, r.mode).await?;
            let row = sqlx::query(
                "SELECT snapshot,status,updated_ms FROM execution.wallets WHERE mode=$1",
            )
            .bind(r.mode.id())
            .fetch_one(&mut *tx)
            .await?;
            ensure!(
                row.get::<String, _>("status") == "BALANCES_CONNECTED"
                    && (0..=15000).contains(&(now_ms() - row.get::<i64, _>("updated_ms"))),
                "Wacht op een actueel OKX-saldo"
            );
            let w: crate::order_wallet::Wallet = serde_json::from_value(
                row.get::<Option<Value>, _>("snapshot")
                    .ok_or_else(|| anyhow::anyhow!("OKX-saldo ontbreekt"))?,
            )?;
            let q = crate::paper::native_quotes(&a).await;
            let q = q
                .get(&r.instrument)
                .ok_or_else(|| anyhow::anyhow!("Verse spotkoers ontbreekt"))?;
            let rules = MarketRules {
                tick: q.tick,
                lot: q.lot,
                minimum: q.minimum,
                base: q.base.clone(),
                quote: q.quote.clone(),
            };
            let snapshot: Option<Value> =
                sqlx::query_scalar("SELECT snapshot FROM execution.accounts WHERE mode=$1")
                    .bind(r.mode.id())
                    .fetch_one(&mut *tx)
                    .await?;
            let fee = snapshot
                .and_then(|v| serde_json::from_value::<AccountSnapshot>(v).ok())
                .filter(|s| s.uid == w.uid && (0..=15000).contains(&(now_ms() - s.received_ms)))
                .and_then(|s| order_risk::account_fee(&s,&r.instrument).ok());
            let cs = claims(&mut tx, r.mode).await?;
            let reserved = cs
                .iter()
                .filter(|c| {
                    c.active
                        && order_risk::claim_currency(c) == if r.side == Direction::Buy { &rules.quote } else { &rules.base }
                })
                .map(|c| {
                    order_risk::claim_amount(c)
                })
                .sum::<D>();
            let sized = crate::order_wallet::size(
                &r,
                &w,
                &rules,
                if r.side == Direction::Buy {
                    q.ask
                } else {
                    q.bid
                },
                fee,
                reserved,
                now_ms(),
            )?;
            tx.commit().await?;
            Ok(serde_json::to_value(sized)?)
        }
        .await,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseRequest {
    account_uid: String,
    policy_revision: i64,
    enabled: bool,
    confirmed: bool,
}
async fn live_release(State(a): State<App>, Json(r): Json<ReleaseRequest>) -> Response {
    reply(async {
        ensure!(r.confirmed,"Bevestig de Live-vrijgave voor dit account en deze limieten");
        let mut tx=a.db.begin().await?;lock(&mut tx,ExecutionMode::Live).await?;
        let row=sqlx::query("SELECT snapshot,policy,policy_revision FROM execution.accounts WHERE mode='LIVE' FOR UPDATE").fetch_one(&mut *tx).await?;
        ensure!(r.policy_revision==row.get::<i64,_>("policy_revision"),"Limieten gewijzigd; controleer opnieuw");
        let s:AccountSnapshot=serde_json::from_value(row.get::<Option<Value>,_>("snapshot").ok_or_else(||anyhow::anyhow!("Live-account ontbreekt"))?)?;
        ensure!(s.uid==r.account_uid,"Account gewijzigd; controleer opnieuw");
        let release=if r.enabled {
            let p:TradingLimits=serde_json::from_value(row.get::<Option<Value>,_>("policy").ok_or_else(||anyhow::anyhow!("Bevestig eerst je handelslimieten"))?)?;
            order_risk::live_ready(&s,&p,now_ms())?;
            Some(serde_json::to_value(order_risk::LiveRelease{account_uid:s.uid,policy_revision:r.policy_revision,confirmed_ms:now_ms()})?)
        }else{None};
        sqlx::query("UPDATE execution.accounts SET live_release=$1,snapshot=jsonb_set(snapshot,'{live_enabled}','false') WHERE mode='LIVE'").bind(&release).execute(&mut *tx).await?;
        if !r.enabled {
            sqlx::query("UPDATE execution.orders SET state='REJECTED',reserve=0,reason='Nieuwe Live-orders gepauzeerd vóór verzending',updated_ms=$1 WHERE mode='LIVE' AND state='QUEUED'").bind(now_ms()).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO execution.events(mode,kind,detail,created_ms) VALUES('LIVE',$1,$2,$3)").bind(if r.enabled{"LIVE_ENABLED"}else{"LIVE_PAUSED"}).bind(json!({"account_uid":r.account_uid,"policy_revision":r.policy_revision})).bind(now_ms()).execute(&mut *tx).await?;
        tx.commit().await?;Ok(json!({"ok":true,"enabled":r.enabled,"message":if r.enabled{"Live vrijgegeven voor jouw account en limieten. Bevestig iedere order apart."}else{"Nieuwe Live-orders gepauzeerd. Bestaande orders en stops blijven bij OKX staan."}}))
    }.await)
}
pub async fn lock(tx: &mut Transaction<'_, Postgres>, mode: ExecutionMode) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(if mode == ExecutionMode::OkxDemo {
            478007_i64
        } else {
            478008_i64
        })
        .execute(&mut **tx)
        .await?;
    Ok(())
}
fn nonce() -> Result<String> {
    use ring::rand::SecureRandom;
    let mut b = [0u8; 12];
    ring::rand::SystemRandom::new()
        .fill(&mut b)
        .map_err(|_| anyhow::anyhow!("Randombron ontbreekt"))?;
    Ok(format!(
        "W{}",
        b.iter().map(|v| format!("{v:02x}")).collect::<String>()
    ))
}
async fn fresh_quote(a: &App, s: &mut AccountSnapshot, t: &Ticket) -> Result<()> {
    let q = crate::paper::native_quotes(a).await;
    let q = q
        .get(&t.instrument)
        .ok_or_else(|| anyhow::anyhow!("Verse gecontroleerde koers ontbreekt"))?;
    let private = s
        .quotes
        .get_mut(&t.instrument)
        .ok_or_else(|| anyhow::anyhow!("Markt nog niet voor account gecontroleerd"))?;
    ensure!(
        private.rules.tick == q.tick
            && private.rules.lot == q.lot
            && private.rules.minimum == q.minimum,
        "Instrumentregels gewijzigd; wacht op accountsynchronisatie"
    );
    private.bid = q.bid;
    private.ask = q.ask;
    private.received_ms = q.received_ms;
    Ok(())
}
async fn status(State(a): State<App>, Query(q): Query<ModeQuery>) -> Response {
    reply(async {
    if let Some(ref instrument)=q.instrument {
        let supported=a.market.read().await.catalogue.iter().any(|i|i["instId"]==*instrument && (i["quoteCcy"]=="EUR" || i["quoteCcy"]=="USDC"));
        if supported { sqlx::query("INSERT INTO execution.market_interest(mode,instrument,updated_ms) VALUES($1,$2,$3) ON CONFLICT(mode,instrument) DO UPDATE SET updated_ms=$3").bind(q.mode.id()).bind(instrument).bind(now_ms()).execute(&a.db).await?; }
    }
    let r=sqlx::query("SELECT snapshot,policy,status,reason,updated_ms,policy_revision,live_release FROM execution.accounts WHERE mode=$1").bind(q.mode.id()).fetch_one(&a.db).await?;
    let snapshot:Option<Value>=r.get("snapshot");let policy:Option<Value>=r.get("policy");
    let credentials_configured=crate::secrets::configured(if q.mode==ExecutionMode::OkxDemo {"okx_demo"}else{"okx_live"}).await;
    let updated=r.get::<i64,_>("updated_ms");let fresh=(0..=15000).contains(&(now_ms()-updated));
    let reason=if !credentials_configured {"Voeg de OKX-key, secret en passphrase toe onder Systeem → Aansluitingen & AI.".to_string()}else if !fresh {"De ordergateway draait nog niet. Start OKX Orders.command opent de afzonderlijke lokale gateway.".into()}else{r.get::<String,_>("reason")};
    Ok(json!({"mode":q.mode,"credentials_configured":credentials_configured,"gateway_fresh":fresh,"status":if fresh {r.get::<String,_>("status")}else{"OFFLINE".into()},"reason":reason,"snapshot":snapshot,"limits":policy,"policy_revision":r.get::<i64,_>("policy_revision"),"profile":"EUR/USDC spot · zonder hefboom","live_technical_accepted":false,"live_release":r.get::<Option<Value>,_>("live_release")}))
}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetLimits {
    mode: ExecutionMode,
    limits: TradingLimits,
}
async fn limits(State(a): State<App>, Json(r): Json<SetLimits>) -> Response {
    reply(async {
    r.limits.validate()?;let mut tx=a.db.begin().await?;lock(&mut tx,r.mode).await?;
    let snapshot:Option<Value>=sqlx::query_scalar("SELECT snapshot FROM execution.accounts WHERE mode=$1 FOR UPDATE").bind(r.mode.id()).fetch_one(&mut *tx).await?;
    let s:AccountSnapshot=serde_json::from_value(snapshot.ok_or_else(||anyhow::anyhow!("Verbind eerst het OKX-account"))?)?;
    ensure!(s.uid==r.limits.account_uid && s.mode==r.mode && s.connected && (0..=15000).contains(&(now_ms()-s.received_ms)),"Bevestig het actuele gedetecteerde account");
    ensure!(r.limits.instruments.iter().all(|id|s.tradable.contains(id)),"Niet alle markten zijn voor deze account gecontroleerd");
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM execution.orders WHERE mode=$1 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(r.mode.id()).fetch_one(&mut *tx).await?;ensure!(count==0,"Annuleer of voltooi eerst de actieve orders voordat je limieten wijzigt");
    sqlx::query("UPDATE execution.accounts SET policy=$2,policy_revision=policy_revision+1,live_release=NULL WHERE mode=$1").bind(r.mode.id()).bind(serde_json::to_value(&r.limits)?).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO execution.events(mode,kind,detail,created_ms) VALUES($1,'LIMITS_CONFIRMED',$2,$3)").bind(r.mode.id()).bind(serde_json::to_value(&r.limits)?).bind(now_ms()).execute(&mut *tx).await?;tx.commit().await?;Ok(json!({"ok":true,"mode":r.mode}))
}.await)
}
pub async fn claims(tx: &mut Transaction<'_, Postgres>, mode: ExecutionMode) -> Result<Vec<Claim>> {
    let rows=sqlx::query("SELECT client_id,ticket,quantity::text,filled::text,reserve::text,created_ms,state FROM execution.orders WHERE mode=$1 AND (state NOT IN ('FILLED','CANCELED','REJECTED') OR created_ms>$2)").bind(mode.id()).bind(now_ms()-60000).fetch_all(&mut **tx).await?;
    let mut claims = rows
        .iter()
        .map(|r| {
            let t: Ticket = serde_json::from_value(r.get("ticket"))?;
            let qty = r.get::<String, _>("quantity").parse::<D>()?;
            let filled = r.get::<String, _>("filled").parse::<D>()?;
            Ok(Claim {
                id: r.get("client_id"),
                instrument: t.instrument,
                side: t.side,
                quantity: (qty - filled).max(D::ZERO),
                reserve: r.get::<String, _>("reserve").parse()?,
                created_ms: r.get("created_ms"),
                active: !["FILLED", "CANCELED", "REJECTED"]
                    .contains(&r.get::<String, _>("state").as_str()),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let protective=sqlx::query("SELECT p.client_id,p.quantity::text,p.created_ms,o.ticket FROM execution.protection p JOIN execution.orders o ON p.mode=o.mode AND p.parent_id=o.client_id WHERE p.mode=$1 AND p.state NOT IN ('FILLED','CANCELED')").bind(mode.id()).fetch_all(&mut **tx).await?;
    for r in protective {
        let t: Ticket = serde_json::from_value(r.get("ticket"))?;
        let q = r.get::<String, _>("quantity").parse::<D>()?;
        claims.push(Claim {
            id: r.get("client_id"),
            instrument: t.instrument,
            side: Direction::Sell,
            quantity: q,
            reserve: q,
            created_ms: r.get("created_ms"),
            active: true,
        });
    }
    Ok(claims)
}
async fn preview(State(a): State<App>, Json(ticket): Json<Ticket>) -> Response {
    reply(async {
    let now=now_ms();let mut tx=a.db.begin().await?;lock(&mut tx,ticket.mode).await?;
    let r=sqlx::query("SELECT snapshot,policy,policy_revision,live_release FROM execution.accounts WHERE mode=$1 FOR UPDATE").bind(ticket.mode.id()).fetch_one(&mut *tx).await?;
    let snapshot:Option<Value>=r.get("snapshot");let policy:Option<Value>=r.get("policy");let id=nonce()?;let client_id=nonce()?;
    // Even before connecting an account, validate against fresh public market metadata.
    // This is a reviewable order draft, never an execution authorization.
    let quotes=crate::paper::native_quotes(&a).await;let q=quotes.get(&ticket.instrument).ok_or_else(||anyhow::anyhow!("Geen actuele gecontroleerde spotkoers; probeer opnieuw"))?;
    let rules=MarketRules{tick:q.tick,lot:q.lot,minimum:q.minimum,base:q.base.clone(),quote:q.quote.clone()};
    let compiled=ticket.compile(&rules,if ticket.side==Direction::Buy{q.ask}else{q.bid},&client_id)?;
    let mut blocked=vec![];let mut risk_plan=None;
    if let (Some(s),Some(p))=(snapshot,policy) {
        let mut s:AccountSnapshot=serde_json::from_value(s)?;fresh_quote(&a,&mut s,&ticket).await?;let p:TradingLimits=serde_json::from_value(p)?;apply_release(&mut s,&p,&r,now)?;let cs=claims(&mut tx,ticket.mode).await?;
        match order_risk::authorize(&ticket,&p,&s,&cs,now,&client_id) {Ok(v)=>risk_plan=Some(json!({"reserve_currency":v.currency(),"reserve":v.reserve().to_string(),"expires_ms":v.expires_ms()})),Err(e)=>blocked.push(e.to_string())}
    }else{blocked.push("Verbind je OKX-account en bevestig de instrumenten, het budget en de limieten.".into());}
    sqlx::query("INSERT INTO execution.previews(id,mode,policy_revision,ticket,compiled,expires_ms,client_id) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(&id).bind(ticket.mode.id()).bind(r.get::<i64,_>("policy_revision")).bind(serde_json::to_value(&ticket)?).bind(serde_json::to_value(&compiled)?).bind(now+15000).bind(&client_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM execution.previews WHERE expires_ms<$1 AND used=false").bind(now-60000).execute(&mut *tx).await?;tx.commit().await?;
    Ok(json!({"preview_id":id,"client_id":client_id,"mode":ticket.mode,"ticket":ticket,"kind":compiled.kind,"price_bound":compiled.worst_price.to_string(),"estimated_notional":(ticket.quantity*compiled.worst_price).to_string(),"can_submit":blocked.is_empty(),"blocked":blocked,"plan":risk_plan,"expires_ms":now+15000,"native":true,"payload":compiled.payload,"warning":"Een marktorder garandeert geen prijs; een triggerprijs is geen uitvoeringsprijs. Preview verzendt niets naar OKX."}))
}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirmation {
    preview_id: String,
    confirmed: bool,
}
async fn submit(State(a): State<App>, Json(r): Json<Confirmation>) -> Response {
    reply(async {
    ensure!(r.confirmed,"Bevestig deze exacte orderpreview");let now=now_ms();let mut tx=a.db.begin().await?;
    let mode:String=sqlx::query_scalar("SELECT mode FROM execution.previews WHERE id=$1").bind(&r.preview_id).fetch_optional(&mut *tx).await?.ok_or_else(||anyhow::anyhow!("Preview ontbreekt"))?;
    let mode:ExecutionMode=serde_json::from_value(json!(mode))?;lock(&mut tx,mode).await?;
    let row=sqlx::query("SELECT * FROM execution.previews WHERE id=$1 FOR UPDATE").bind(&r.preview_id).fetch_one(&mut *tx).await?;
    let client_id:String=row.get("client_id");if row.get::<bool,_>("used") {return Ok(json!({"client_id":client_id,"duplicate":true}));}
    ensure!(now<=row.get::<i64,_>("expires_ms"),"Preview verlopen; maak een nieuwe preview");
    let account=sqlx::query("SELECT snapshot,policy,policy_revision,live_release FROM execution.accounts WHERE mode=$1 FOR UPDATE").bind(mode.id()).fetch_one(&mut *tx).await?;
    ensure!(row.get::<i64,_>("policy_revision")==account.get::<i64,_>("policy_revision"),"Limieten gewijzigd; nieuwe preview vereist");
    let mut s:AccountSnapshot=serde_json::from_value(account.get::<Option<Value>,_>("snapshot").ok_or_else(||anyhow::anyhow!("OKX-account ontbreekt"))?)?;
    let p:TradingLimits=serde_json::from_value(account.get::<Option<Value>,_>("policy").ok_or_else(||anyhow::anyhow!("Limieten ontbreken"))?)?;
    apply_release(&mut s,&p,&account,now)?;let t:Ticket=serde_json::from_value(row.get("ticket"))?;fresh_quote(&a,&mut s,&t).await?;let cs=claims(&mut tx,mode).await?;let accepted=order_risk::authorize(&t,&p,&s,&cs,now,&client_id)?;
    let old:CompiledOrder=serde_json::from_value(row.get("compiled"))?;
    ensure!(t.side!=Direction::Buy || accepted.compiled().worst_price<=old.worst_price,"Prijs verslechterd; nieuwe preview vereist");
    sqlx::query("INSERT INTO execution.orders(mode,client_id,account_uid,ticket,compiled,state,reserve_currency,reserve,quantity,created_ms,updated_ms) VALUES($1,$2,$3,$4,$5,'QUEUED',$6,$7::numeric,$8::numeric,$9,$9)").bind(mode.id()).bind(&client_id).bind(&s.uid).bind(serde_json::to_value(&t)?).bind(serde_json::to_value(accepted.compiled())?).bind(accepted.currency()).bind(accepted.reserve().to_string()).bind(t.quantity.to_string()).bind(now).execute(&mut *tx).await?;
    sqlx::query("UPDATE execution.previews SET used=true WHERE id=$1").bind(&r.preview_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO execution.events(mode,client_id,kind,detail,created_ms) VALUES($1,$2,'QUEUED',$3,$4)").bind(mode.id()).bind(&client_id).bind(json!({"preview_id":r.preview_id})).bind(now).execute(&mut *tx).await?;tx.commit().await?;
    Ok(json!({"mode":mode,"client_id":client_id,"state":"QUEUED","message":"Duurzaam aangeboden aan de ordergateway; dit is nog geen exchange-ACK of fill."}))
}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cancel {
    mode: ExecutionMode,
    client_id: String,
}
async fn cancel(State(a): State<App>, Json(r): Json<Cancel>) -> Response {
    reply(async {
    let mut tx=a.db.begin().await?;lock(&mut tx,r.mode).await?;let row=sqlx::query("SELECT state FROM execution.orders WHERE mode=$1 AND client_id=$2 FOR UPDATE").bind(r.mode.id()).bind(&r.client_id).fetch_optional(&mut *tx).await?.ok_or_else(||anyhow::anyhow!("Order ontbreekt in deze omgeving"))?;
    let state:String=row.get("state");ensure!(!["FILLED","CANCELED","REJECTED"].contains(&state.as_str()),"Order is al definitief");
    if state=="QUEUED" {sqlx::query("UPDATE execution.orders SET state='CANCELED',reserve=0,reason='Geannuleerd vóór verzending',updated_ms=$3 WHERE mode=$1 AND client_id=$2").bind(r.mode.id()).bind(&r.client_id).bind(now_ms()).execute(&mut *tx).await?;}else{sqlx::query("UPDATE execution.orders SET command='CANCEL',updated_ms=$3 WHERE mode=$1 AND client_id=$2").bind(r.mode.id()).bind(&r.client_id).bind(now_ms()).execute(&mut *tx).await?;}
    tx.commit().await?;Ok(json!({"ok":true,"mode":r.mode,"client_id":r.client_id,"message":"Annulering aangevraagd. Reserveringen verdwijnen pas na een definitieve OKX-status."}))
}.await)
}
#[derive(Deserialize)]
struct OrderQuery {
    mode: ExecutionMode,
    #[serde(default)]
    before: Option<i64>,
}
async fn orders(State(a): State<App>, Query(q): Query<OrderQuery>) -> Response {
    reply(async {
    let rows=sqlx::query("SELECT mode,client_id,ticket,state,exchange_id,reason,quantity::text,filled::text,created_ms,updated_ms FROM execution.orders WHERE mode=$1 AND created_ms<$2 ORDER BY created_ms DESC LIMIT 100").bind(q.mode.id()).bind(q.before.unwrap_or(i64::MAX)).fetch_all(&a.db).await?;
    Ok(json!({"orders":rows.iter().map(|r|json!({"mode":r.get::<String,_>("mode"),"client_id":r.get::<String,_>("client_id"),"ticket":r.get::<Value,_>("ticket"),"state":r.get::<String,_>("state"),"exchange_id":r.get::<Option<String>,_>("exchange_id"),"reason":r.get::<String,_>("reason"),"quantity":r.get::<String,_>("quantity"),"filled":r.get::<String,_>("filled"),"created_ms":r.get::<i64,_>("created_ms"),"updated_ms":r.get::<i64,_>("updated_ms")})).collect::<Vec<_>>(),"next_before":rows.last().map(|r|r.get::<i64,_>("created_ms"))}))
}.await)
}
async fn activity(State(a): State<App>, Query(q): Query<ModeQuery>) -> Response {
    reply(async{
    let fills=sqlx::query("SELECT bill_id,parent_id,instrument,side,quantity::text,price::text,fee::text,fee_currency,fill_ms FROM execution.fills WHERE mode=$1 ORDER BY fill_ms DESC,bill_id DESC LIMIT 100").bind(q.mode.id()).fetch_all(&a.db).await?;
    let protects=sqlx::query("SELECT parent_id,client_id,quantity::text,state,exchange_id,created_ms FROM execution.protection WHERE mode=$1 ORDER BY created_ms DESC LIMIT 100").bind(q.mode.id()).fetch_all(&a.db).await?;
    let events=sqlx::query("SELECT id,client_id,kind,detail,created_ms FROM execution.events WHERE mode=$1 ORDER BY id DESC LIMIT 30").bind(q.mode.id()).fetch_all(&a.db).await?;
    Ok(json!({"fills":fills.iter().map(|r|json!({"bill_id":r.get::<String,_>("bill_id"),"parent_id":r.get::<String,_>("parent_id"),"instrument":r.get::<String,_>("instrument"),"side":r.get::<String,_>("side"),"quantity":r.get::<String,_>("quantity"),"price":r.get::<String,_>("price"),"fee":r.get::<String,_>("fee"),"fee_currency":r.get::<String,_>("fee_currency"),"fill_ms":r.get::<i64,_>("fill_ms")})).collect::<Vec<_>>(),"protection":protects.iter().map(|r|json!({"parent_id":r.get::<String,_>("parent_id"),"client_id":r.get::<String,_>("client_id"),"quantity":r.get::<String,_>("quantity"),"state":r.get::<String,_>("state"),"exchange_id":r.get::<Option<String>,_>("exchange_id"),"created_ms":r.get::<i64,_>("created_ms")})).collect::<Vec<_>>(),"events":events.iter().map(|r|json!({"id":r.get::<i64,_>("id"),"kind":r.get::<String,_>("kind"),"detail":r.get::<Value,_>("detail"),"created_ms":r.get::<i64,_>("created_ms")})).collect::<Vec<_>>()}))
}.await)
}
