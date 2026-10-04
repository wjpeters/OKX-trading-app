//! Local control plane + single paper worker. All money mutations lock the same DB row.
use crate::{
    App, CandleQuery, candle_result,
    model::{Candle, now_ms},
    risk::*,
};
use anyhow::{Result, ensure};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rust_decimal::Decimal as D;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use std::{collections::BTreeMap, sync::atomic::Ordering, time::Duration};

pub async fn quotes(app: &App) -> Quotes {
    let m = app.market.read().await;
    let mut out = Quotes::new();
    if !m.feed_connected
        || m.metadata_received_ms
            .is_none_or(|t| !(0..=330000).contains(&(now_ms() - t)))
        || m.clock_offset_ms.is_none_or(|x| x.abs() > 1000)
        || m.clock_rtt_ms.is_none_or(|x| x > 2000)
    {
        return out;
    }
    for i in &m.instruments {
        if i["quoteCcy"] != "EUR" || i["state"] != "live" {
            continue;
        }
        let Some(id) = i["instId"].as_str() else {
            continue;
        };
        let Some(b) = m.books.get(id) else { continue };
        let parse = |v: &Value| v.as_str().and_then(|x| x.parse::<D>().ok());
        let Some((bid, ask, bs, asz, tick, lot, minimum)) = parse(&b["bids"][0][0])
            .zip(parse(&b["asks"][0][0]))
            .zip(parse(&b["bids"][0][1]))
            .zip(parse(&b["asks"][0][1]))
            .zip(parse(&i["tickSz"]))
            .zip(parse(&i["lotSz"]))
            .zip(parse(&i["minSz"]))
            .map(|((((((a, b), c), d), e), f), g)| (a, b, c, d, e, f, g))
        else {
            continue;
        };
        let Some(ts) = b["ts"].as_str().and_then(|x| x.parse::<i64>().ok()) else {
            continue;
        };
        out.insert(
            id.into(),
            Quote {
                instrument: id.into(),
                base: i["baseCcy"].as_str().unwrap_or_default().into(),
                quote: "EUR".into(),
                bid,
                ask,
                bid_size: bs,
                ask_size: asz,
                tick,
                lot,
                minimum,
                ts,
                received_ms: b["received_at_ms"].as_i64().unwrap_or(0),
            },
        );
    }
    out
}
pub(crate) async fn load(tx: &mut Transaction<'_, Postgres>) -> Result<Paper> {
    let row = sqlx::query("SELECT payload FROM paper_v2.account WHERE id=1 FOR UPDATE")
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Stel eerst de virtuele paperrekening in"))?;
    let p: Paper = serde_json::from_value(row.get("payload"))?;
    ensure!(
        p.schema == 2 && p.mode == "PAPER_LOCAL",
        "Opslagnamespace/modus ongeldig"
    );
    Ok(p)
}
pub(crate) async fn persist(tx: &mut Transaction<'_, Postgres>, p: &mut Paper) -> Result<()> {
    let stored: i64 = sqlx::query_scalar("SELECT revision FROM paper_v2.account WHERE id=1")
        .fetch_one(&mut **tx)
        .await?;
    for e in p.events.iter().filter(|e| e.seq as i64 > stored) {
        sqlx::query("INSERT INTO paper_v2.events(seq,ts,kind,subject,detail) VALUES($1,$2,$3,$4,$5) ON CONFLICT(seq) DO NOTHING").bind(e.seq as i64).bind(e.ts).bind(&e.kind).bind(&e.subject).bind(&e.detail).execute(&mut **tx).await?;
    }
    if p.events.len() > 100 {
        p.events.drain(..p.events.len() - 100);
    }
    sqlx::query("UPDATE paper_v2.account SET revision=$1,payload=$2,updated_at=now() WHERE id=1")
        .bind(p.revision as i64)
        .bind(serde_json::to_value(p)?)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
fn response(r: Result<Value>) -> Response {
    match r {
        Ok(v) => Json(v).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":e.to_string()})),
        )
            .into_response(),
    }
}
pub fn router() -> Router<App> {
    Router::new()
        .route("/api/paper", get(state))
        .route("/api/paper/setup", post(setup))
        .route("/api/paper/preview", post(preview))
        .route("/api/paper/submit", post(submit))
        .route("/api/paper/cancel", post(cancel))
        .route("/api/paper/control", post(control))
        .route("/api/paper/bots", post(add_bot))
        .route("/api/paper/bots/control", post(bot_control))
        .route("/api/replay", post(replay))
        .route("/api/replay/evaluate", post(evaluate))
        .route("/api/replay/pine", post(pine_replay))
        .route("/api/replay/reports", get(reports))
}
async fn state(State(app): State<App>) -> Response {
    response(async {
    let row=sqlx::query("SELECT payload FROM paper_v2.account WHERE id=1").fetch_optional(&app.db).await?;
    let payload:Option<Value>=row.map(|r|r.get("payload"));
    Ok(json!({"account":payload,"presets":{"account":Limits::preset(d("10000")),"manual":Limits::preset(d("1000"))},"assumptions":{"fee_bps":FEE_BPS,"latency_ms":LATENCY_MS,"participation_percent":10,"timezone":"UTC","quote_currency":"EUR","price_source":"OKX EEA books5","leverage":"1","funding":"0","ai_cost":"0","cashflows":"initial virtual deposit only"},"worker_alive":now_ms()-app.paper_heartbeat.load(Ordering::Relaxed)<3000,"server_time_ms":now_ms()}))
}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Setup {
    account: Limits,
    manual: Limits,
    confirmed: bool,
}
async fn setup(State(app): State<App>, Json(r): Json<Setup>) -> Response {
    response(async {
    ensure!(r.confirmed,"Bevestig virtueel kapitaal, alle limieten, UTC-daggrens en kostenmodel");
    let p=Paper::new(r.account,r.manual,now_ms())?;
    let mut tx=app.db.begin().await?;
    ensure!(sqlx::query("INSERT INTO paper_v2.account(id,revision,payload) VALUES(1,$1,$2) ON CONFLICT DO NOTHING").bind(0_i64).bind(serde_json::to_value(&p)?).execute(&mut *tx).await?.rows_affected()==1,"Paperrekening bestaat al; geen reset of saldo-overschrijving toegestaan");
    let mut p=p;persist(&mut tx,&mut p).await?;tx.commit().await?;
    Ok(json!({"ok":true,"mode":"PAPER_LOCAL"}))
}.await)
}
async fn preview(State(app): State<App>, Json(i): Json<Intent>) -> Response {
    response(async {
    ensure!(i.side==Side::Sell||i.bot_id=="manual","Handmatige instappen gebruiken uitsluitend het handmatige deelbudget");
    let qs=quotes(&app).await;
    let q=qs.get(&i.instrument).ok_or_else(||anyhow::anyhow!("Geen verse EUR-marktdata/kloksynchronisatie"))?;
    let mut tx=app.db.begin().await?;let mut p=load(&mut tx).await?;let now=now_ms();p.mark(&qs,now);
    let plan=p.preview(&i,q,now)?;
    sqlx::query("DELETE FROM paper_v2.previews WHERE expires_ms<$1").bind(now-60000).execute(&mut *tx).await?;
    let id:i64=sqlx::query_scalar("INSERT INTO paper_v2.previews(revision,expires_ms,payload) VALUES($1,$2,$3) RETURNING id").bind(p.revision as i64).bind(now+15000).bind(json!({"intent":i,"plan":plan})).fetch_one(&mut *tx).await?;
    persist(&mut tx,&mut p).await?;tx.commit().await?;
    Ok(json!({"preview_id":id,"expires_ms":now+15000,"plan":plan,"intent":i}))
}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirm {
    preview_id: i64,
    confirmed: bool,
}
async fn submit(State(app): State<App>, Json(r): Json<Confirm>) -> Response {
    response(async {
    ensure!(r.confirmed,"Paperorder moet expliciet bevestigd worden");
    ensure!(now_ms()-app.paper_heartbeat.load(Ordering::Relaxed)<3000,"Paperworker niet beschikbaar");
    let start=std::time::Instant::now();
    let qs=quotes(&app).await;let mut tx=app.db.begin().await?;
    let mut p=load(&mut tx).await?;let now=now_ms();p.mark(&qs,now);
    let row=sqlx::query("SELECT revision,expires_ms,payload,used FROM paper_v2.previews WHERE id=$1 FOR UPDATE").bind(r.preview_id).fetch_optional(&mut *tx).await?.ok_or_else(||anyhow::anyhow!("Preview ontbreekt"))?;
    let payload:Value=row.get("payload");let i:Intent=serde_json::from_value(payload["intent"].clone())?;
    // Retry of the same confirmed preview returns the same order, never another order.
    if row.get::<bool,_>("used") {let old=p.orders.values().find(|o|o.intent.client_id==i.client_id).ok_or_else(||anyhow::anyhow!("Preview/order inconsistent"))?;return Ok(json!({"order_id":old.id,"duplicate":true}));}
    ensure!(now<row.get::<i64,_>("expires_ms"),"Preview verlopen; opnieuw beoordelen");
    ensure!(p.revision as i64==row.get::<i64,_>("revision"),"Rekeningstate gewijzigd; nieuwe preview vereist");
    let q=qs.get(&i.instrument).ok_or_else(||anyhow::anyhow!("Markt niet beschikbaar"))?;
    let current=p.preview(&i,q,now)?;
    let num=|v:&Value|v.as_str().and_then(|s|s.parse::<D>().ok()).ok_or_else(||anyhow::anyhow!("Ongeldige previewprijs"));
    let bound=num(&payload["plan"]["price_bound"])?;let next=num(&current["price_bound"])?;
    ensure!((i.side==Side::Buy&&next<=bound)||(i.side==Side::Sell&&next>=bound),"Prijsgrens verslechterd; nieuwe preview vereist");
    let id=p.submit(i,q,now)?;
    if let Some(pid)=p.orders[&id].intent.position_id.clone() {p.positions.get_mut(&pid).unwrap().closing_order=Some(id.clone());}
    sqlx::query("UPDATE paper_v2.previews SET used=true WHERE id=$1").bind(r.preview_id).execute(&mut *tx).await?;
    persist(&mut tx,&mut p).await?;tx.commit().await?;
    Ok(json!({"order_id":id,"state":"PENDING_SUBMIT","durable_ms":start.elapsed().as_secs_f64()*1000.0}))
}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cancel {
    order_id: String,
}
async fn cancel(State(app): State<App>, Json(r): Json<Cancel>) -> Response {
    response(
        async {
            let mut tx = app.db.begin().await?;
            let mut p = load(&mut tx).await?;
            p.request_cancel(&r.order_id, now_ms())?;
            persist(&mut tx, &mut p).await?;
            tx.commit().await?;
            Ok(json!({"ok":true}))
        }
        .await,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    action: String,
    confirmed: bool,
}
async fn control(State(app): State<App>, Json(r): Json<Control>) -> Response {
    response(
        async {
            ensure!(r.confirmed, "Bevestiging vereist");
            let qs = quotes(&app).await;
            let mut tx = app.db.begin().await?;
            let mut p = load(&mut tx).await?;
            p.control(&r.action, &qs, now_ms())?;
            persist(&mut tx, &mut p).await?;
            tx.commit().await?;
            Ok(json!({"ok":true}))
        }
        .await,
    )
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotConfig {
    pub id: String,
    pub name: String,
    pub instrument: String,
    pub limits: Limits,
    pub fast: usize,
    pub slow: usize,
    pub order_eur: D,
    pub stop_bps: D,
    pub target_bps: D,
}
impl BotConfig {
    fn bot(self, now: i64) -> Result<Bot> {
        let seed = Paper::new(self.limits.clone(), self.limits.clone(), now)?;
        Ok(Bot {
            id: self.id,
            name: self.name,
            instrument: self.instrument,
            version: "sma-cross-v1".into(),
            limits: self.limits,
            ledger: seed.ledger,
            running: false,
            last_bar: 0,
            last_signal: String::new(),
            fast: self.fast,
            slow: self.slow,
            order_eur: self.order_eur,
            stop_bps: self.stop_bps,
            target_bps: self.target_bps,
        })
    }
}
async fn add_bot(State(app): State<App>, Json(r): Json<BotConfig>) -> Response {
    response(
        async {
            let qs = quotes(&app).await;
            ensure!(
                qs.contains_key(&r.instrument),
                "Kies een beschikbare EUR-spotmarkt"
            );
            let mut tx = app.db.begin().await?;
            let mut p = load(&mut tx).await?;
            let now = now_ms();
            p.add_bot(r.bot(now)?, now)?;
            persist(&mut tx, &mut p).await?;
            tx.commit().await?;
            Ok(json!({"ok":true}))
        }
        .await,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BotControl {
    bot_id: String,
    running: bool,
}
async fn bot_control(State(app): State<App>, Json(r): Json<BotControl>) -> Response {
    response(
        async {
            let qs = quotes(&app).await;
            let mut tx = app.db.begin().await?;
            let mut p = load(&mut tx).await?;
            p.bot_control(&r.bot_id, r.running, &qs, now_ms())?;
            persist(&mut tx, &mut p).await?;
            tx.commit().await?;
            Ok(json!({"ok":true}))
        }
        .await,
    )
}
async fn run_bots(app: &App) -> Result<()> {
    let Some(value) =
        sqlx::query_scalar::<_, Value>("SELECT payload FROM paper_v2.account WHERE id=1")
            .fetch_optional(&app.db)
            .await?
    else {
        return Ok(());
    };
    let initial: Paper = serde_json::from_value(value)?;
    for bot in initial
        .bots
        .values()
        .filter(|b| b.id != "manual" && b.running)
    {
        let data = match candle_result(
            app,
            &CandleQuery {
                instrument: bot.instrument.clone(),
                bar: "1m".into(),
                after: None,
            },
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                record_bot_error(app, &bot.id, &format!("CANDLE_UNAVAILABLE: {e}")).await?;
                continue;
            }
        };
        if data["gaps"] != 0 || data["fresh"] != true {
            record_bot_error(app, &bot.id, "CANDLE_GAP_OR_STALE").await?;
            continue;
        }
        let cs: Vec<Candle> = serde_json::from_value(data["candles"].clone())?;
        let closed: Vec<_> = cs
            .iter()
            .filter(|c| c.closed && c.ts + 60000 <= now_ms())
            .collect();
        let Some(last) = closed.last() else { continue };
        if last.ts <= bot.last_bar {
            continue;
        }
        let graph_signal = if bot.version.starts_with("graph:") {
            match crate::graph::signal(app, bot, &data, initial.revision).await {
                Ok(s) => Some(s),
                Err(e) => {
                    record_bot_error(app, &bot.id, &format!("PLAN_BLOCKED: {e}")).await?;
                    continue;
                }
            }
        } else {
            None
        };
        let qs = quotes(app).await;
        let now = now_ms();
        let mut tx = app.db.begin().await?;
        let mut p = load(&mut tx).await?;
        let Some(b) = p.bots.get(&bot.id).cloned() else {
            continue;
        };
        if !b.running || b.version != bot.version || last.ts <= b.last_bar {
            continue;
        }
        p.mark(&qs, now);
        let signal = if last.ts + 75000 < now {
            "STALE_CLOSE"
        } else if let Some(ref s) = graph_signal {
            s.as_str()
        } else {
            sma_signal(
                &closed.iter().map(|c| c.close).collect::<Vec<_>>(),
                b.fast,
                b.slow,
            )
            .unwrap_or("WARMUP")
        };
        let outcome = process_signal(&mut p, &b, signal, &qs, last.ts, now);
        let b = p.bots.get_mut(&bot.id).unwrap();
        b.last_bar = last.ts;
        b.last_signal = outcome.clone();
        p.event(
            now,
            "BOT_DECISION",
            &bot.id,
            &format!("Candle {}: {outcome}", last.ts),
        );
        persist(&mut tx, &mut p).await?;
        tx.commit().await?;
    }
    Ok(())
}
fn process_signal(p: &mut Paper, b: &Bot, signal: &str, qs: &Quotes, bar: i64, now: i64) -> String {
    let Some(q) = qs.get(&b.instrument) else {
        return "Geen verse marktdata".into();
    };
    let positions: Vec<_> = p
        .positions
        .values()
        .filter(|pos| pos.bot_id == b.id && pos.quantity > D::ZERO)
        .cloned()
        .collect();
    if signal == "BUY"
        && positions.is_empty()
        && !p
            .orders
            .values()
            .any(|o| o.intent.bot_id == b.id && o.state.open())
    {
        return match p.submit(strategy_intent(b, q, bar), q, now) {
            Ok(id) => format!("BUY → {id}"),
            Err(e) => format!("Afgewezen: {e}"),
        };
    }
    if signal == "SELL" {
        for pos in positions {
            if pos.closing_order.is_some() {
                continue;
            }
            let i = Intent {
                client_id: format!("{}-exit-{bar}", b.id),
                bot_id: b.id.clone(),
                instrument: b.instrument.clone(),
                side: Side::Sell,
                kind: Kind::Market,
                quantity: pos.quantity,
                limit: None,
                stop: None,
                target: None,
                position_id: Some(pos.id.clone()),
            };
            match p.submit(i, q, now) {
                Ok(id) => {
                    p.positions.get_mut(&pos.id).unwrap().closing_order = Some(id);
                }
                Err(e) => return format!("Afbouw afgewezen: {e}"),
            }
        }
    }
    signal.into()
}
async fn record_bot_error(app: &App, id: &str, message: &str) -> Result<()> {
    let mut tx = app.db.begin().await?;
    let mut p = load(&mut tx).await?;
    if let Some(b) = p.bots.get_mut(id) {
        if b.last_signal != message {
            b.last_signal = message.into();
            p.event(now_ms(), "BOT_DATA_BLOCK", id, message);
            persist(&mut tx, &mut p).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
pub async fn run(app: App) {
    loop {
        if let Err(e) = worker(&app).await {
            tracing::warn!(error=%e,"Paperworker paused; no uncommitted execution");
        }
        app.paper_heartbeat.store(0, Ordering::Relaxed);
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
async fn worker(app: &App) -> Result<()> {
    // Session ownership; losing this connection stops the worker. Row-lock transactions also
    // prevent duplicate effects if another process resumes during a network interruption.
    let mut lease = app.db.acquire().await?;
    let owner: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(478002)")
        .fetch_one(&mut *lease)
        .await?;
    ensure!(owner, "Andere paperworker bezit de rekening");
    let result = async {
        let qs = quotes(app).await;
        let mut tx = app.db.begin().await?;
        if let Ok(mut p) = load(&mut tx).await {
            p.recover(&qs, now_ms());
            persist(&mut tx, &mut p).await?;
        }
        tx.commit().await?;
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            sqlx::query("SELECT 1").execute(&mut *lease).await?;
            let qs = quotes(app).await;
            let mut tx = app.db.begin().await?;
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM paper_v2.account WHERE id=1)")
                    .fetch_one(&mut *tx)
                    .await?;
            if exists {
                let mut p = load(&mut tx).await?;
                p.tick(&qs, now_ms());
                persist(&mut tx, &mut p).await?;
            }
            tx.commit().await?;
            app.paper_heartbeat.store(now_ms(), Ordering::Relaxed);
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    }
    .await;
    // Never return an advisory-lock-bearing pooled connection for arbitrary reuse.
    sqlx::query("SELECT pg_advisory_unlock(478002)")
        .execute(&mut *lease)
        .await
        .ok();
    lease.close().await.ok();
    result
}
pub async fn bots(app: App) {
    loop {
        if app.paper_heartbeat.load(Ordering::Relaxed) > now_ms() - 3000 {
            if let Err(e) = run_bots(&app).await {
                tracing::warn!(error=%e,"Botcyclus uitgesteld");
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplayRequest {
    instrument: String,
    bar: String,
    fast: usize,
    slow: usize,
    capital: D,
    order_eur: D,
    stop_bps: D,
    target_bps: D,
    spread_bps: D,
    slippage_bps: D,
}
async fn replay(State(app): State<App>, Json(r): Json<ReplayRequest>) -> Response {
    response(
        async {
            let _permit = app
                .replays
                .clone()
                .try_acquire_owned()
                .map_err(|_| anyhow::anyhow!("Er draait al een replay"))?;
            ensure!(
                r.spread_bps >= d("1")
                    && r.spread_bps <= d("30")
                    && r.slippage_bps >= D::ZERO
                    && r.slippage_bps <= d("20"),
                "Spread 1–30 en slippage 0–20 basispunten vereist"
            );
            let q = quotes(&app)
                .await
                .remove(&r.instrument)
                .ok_or_else(|| anyhow::anyhow!("EUR-instrumentmetadata niet beschikbaar"))?;
            let data = candle_result(
                &app,
                &CandleQuery {
                    instrument: r.instrument.clone(),
                    bar: r.bar.clone(),
                    after: None,
                },
            )
            .await?;
            ensure!(data["gaps"] == 0, "Replay gestopt: dataset bevat gaten");
            let candles: Vec<Candle> = serde_json::from_value(data["candles"].clone())?;
            let closed: Vec<_> = candles
                .into_iter()
                .filter(|c| {
                    c.closed
                        && c.ts + crate::model::interval(&r.bar).unwrap_or(i64::MAX) <= now_ms()
                })
                .collect();
            let report = replay_candles(&closed, &q, &r)?;
            let id: i64 =
                sqlx::query_scalar("INSERT INTO paper_v2.replays(report) VALUES($1) RETURNING id")
                    .bind(&report)
                    .fetch_one(&app.db)
                    .await?;
            Ok(json!({"id":id,"report":report}))
        }
        .await,
    )
}
#[derive(Deserialize)]
struct ReportsQuery {
    mode: Option<String>,
}
async fn reports(State(app): State<App>, Query(q): Query<ReportsQuery>) -> Response {
    response(
        async {
            let mode=q.mode.as_deref().unwrap_or("REPLAY");
            ensure!(["REPLAY","PINE_REPLAY","WALK_FORWARD"].contains(&mode),"Rapporttype ongeldig");
            let rows =
                sqlx::query("SELECT id,report FROM paper_v2.replays WHERE report->>'mode'=$1 ORDER BY id DESC LIMIT 20")
                    .bind(mode)
                    .fetch_all(&app.db)
                    .await?;
            Ok(json!(
                rows.iter()
                    .map(|r| json!({"id":r.get::<i64,_>("id"),"report":r.get::<Value,_>("report")}))
                    .collect::<Vec<_>>()
            ))
        }
        .await,
    )
}
fn replay_candles(cs: &[Candle], meta: &Quote, r: &ReplayRequest) -> Result<Value> {
    replay_signals(cs, meta, r, None)
}
fn replay_signals(
    cs: &[Candle],
    meta: &Quote,
    r: &ReplayRequest,
    signals: Option<&BTreeMap<i64, String>>,
) -> Result<Value> {
    ensure!(
        r.spread_bps >= d("1")
            && r.spread_bps <= d("30")
            && r.slippage_bps >= D::ZERO
            && r.slippage_bps <= d("20"),
        "Ongeldige kostenparameters"
    );

    ensure!(
        r.fast >= 2 && r.fast < r.slow && r.slow <= 100,
        "SMA-parameters buiten bereik"
    );
    ensure!(
        r.capital >= d("10") && r.capital <= d("1000000"),
        "Replaykapitaal buiten bereik"
    );
    ensure!(
        cs.len() > r.slow + 1,
        "Onvoldoende gesloten candles voor warm-up"
    );
    let step =
        crate::model::interval(&r.bar).ok_or_else(|| anyhow::anyhow!("Timeframe ongeldig"))?;
    ensure!(
        cs.windows(2).all(|w| w[1].ts - w[0].ts == step),
        "Dataset niet aaneengesloten"
    );
    let mut limits = Limits::preset(r.capital);
    limits.orders_per_minute = 60;
    let mut p = Paper::new(limits.clone(), limits.clone(), cs[0].ts)?;
    p.mode = "REPLAY".into();
    p.bots.clear();
    let config = BotConfig {
        id: "replay".into(),
        name: "SMA replay".into(),
        instrument: r.instrument.clone(),
        limits,
        fast: r.fast,
        slow: r.slow,
        order_eur: r.order_eur,
        stop_bps: r.stop_bps,
        target_bps: r.target_bps,
    };
    p.add_bot(config.bot(cs[0].ts)?, cs[0].ts)?;
    p.bots.get_mut("replay").unwrap().running = true;
    let mut closes = vec![];
    let mut curve = vec![];
    let mut max_drawdown = D::ZERO;
    let mut max_exposure = D::ZERO;
    let mut decisions = vec![];
    let mut qs = BTreeMap::new();
    let quote = |price: D, ts: i64, c: &Candle| {
        let mut q = meta.clone();
        q.ts = ts;
        q.received_ms = ts;
        let half = r.spread_bps / d("20000") + r.slippage_bps / d("10000");
        q.bid = ((price * (D::ONE - half)) / q.tick).floor() * q.tick;
        q.ask = ((price * (D::ONE + half)) / q.tick).ceil() * q.tick;
        q.bid_size = c.volume / d("5");
        q.ask_size = c.volume / d("5");
        q
    };
    for c in cs {
        // OHLC path is an explicit conservative assumption: open -> low -> high -> close.
        // No signal based on this close can fill before the next candle opens.
        for (offset, price) in [
            (LATENCY_MS, c.open),
            (LATENCY_MS + 1, c.open),
            (1500, c.low),
            (2000, c.low),
            (2500, c.low),
            (3000, c.high),
            (3500, c.high),
            (4000, c.high),
            (step - 1, c.close),
        ] {
            let phase = if offset <= LATENCY_MS + 1 {
                0
            } else if offset <= 2500 {
                1500
            } else if offset <= 4000 {
                3000
            } else {
                step - 1
            };
            let q = quote(price, c.ts + phase, c);
            qs.insert(r.instrument.clone(), q);
            p.tick(&qs, c.ts + offset);
            max_drawdown = max_drawdown.max(p.ledger.drawdown);
            max_exposure = max_exposure.max(
                p.positions
                    .values()
                    .map(|pos| pos.quantity * qs[&pos.instrument].bid)
                    .sum(),
            );
        }
        closes.push(c.close);
        let b = p.bots["replay"].clone();
        let signal = if let Some(s) = signals {
            s.get(&c.ts).map(String::as_str).unwrap_or("HOLD")
        } else {
            sma_signal(&closes, b.fast, b.slow).unwrap_or("WARMUP")
        };
        let outcome = process_signal(&mut p, &b, signal, &qs, c.ts, c.ts + step - 1);
        if signal != "HOLD" && signal != "WARMUP" {
            decisions.push(json!({"ts":c.ts,"signal":signal,"outcome":outcome}));
        }
        curve.push(json!({"ts":c.ts,"equity":p.ledger.equity,"drawdown":p.ledger.drawdown}));
    }
    let turnover: D = p.fills.iter().map(|f| f.quantity * f.price).sum();
    Ok(
        json!({"mode":"REPLAY","strategy":"sma-cross-v1","instrument":r.instrument,"bar":r.bar,"bars":cs.len(),"first_ms":cs.first().unwrap().ts,"last_ms":cs.last().unwrap().ts,"parameters":{"fast":r.fast,"slow":r.slow,"capital":r.capital,"order_eur":r.order_eur,"stop_bps":r.stop_bps,"target_bps":r.target_bps},"assumptions":{"type":"CANDLE_OHLC_CONSERVATIVE","path":"open → low → high → close; stop vóór target","spread_bps":r.spread_bps,"slippage_bps":r.slippage_bps,"fee_bps":FEE_BPS,"participation":"10% van geschat volume/5 per prijsfase; geen echte queue","latency_ms":LATENCY_MS,"history":"Werkelijk geladen gesloten OKX-candles; bereik en aantal staan in dit rapport","uncertainty":"OHLC onthult geen tickvolgorde of liquiditeit; geen winstbewijs of echte accountfees"},"equity":p.ledger.equity,"net_pnl":p.ledger.equity-r.capital,"realized":p.ledger.realized,"fees":p.ledger.fees,"max_drawdown":max_drawdown,"max_exposure":max_exposure,"turnover":turnover,"open_positions":p.positions.values().filter(|p|p.quantity>D::ZERO).count(),"fill_count":p.fills.len(),"risk_state":p.ledger.reason,"curve":curve,"fills":p.fills,"decisions":decisions,"limits":p.limits}),
    )
}

#[cfg(test)]
mod replay_tests {
    use super::*;
    fn request() -> ReplayRequest {
        ReplayRequest {
            instrument: "BTC-EUR".into(),
            bar: "1m".into(),
            fast: 2,
            slow: 3,
            capital: d("1000"),
            order_eur: d("100"),
            stop_bps: d("100"),
            target_bps: d("200"),
            spread_bps: d("10"),
            slippage_bps: d("5"),
        }
    }
    fn meta() -> Quote {
        Quote {
            instrument: "BTC-EUR".into(),
            base: "BTC".into(),
            quote: "EUR".into(),
            bid: d("100"),
            ask: d("100.1"),
            bid_size: d("1000"),
            ask_size: d("1000"),
            tick: d("0.01"),
            lot: d("0.01"),
            minimum: d("0.01"),
            ts: 0,
            received_ms: 0,
        }
    }
    fn candles() -> Vec<Candle> {
        ["103", "102", "101", "108", "108", "108", "108"]
            .iter()
            .enumerate()
            .map(|(n, c)| {
                let price = d(c);
                Candle {
                    ts: 1_699_999_980_000 + n as i64 * 60000,
                    open: price,
                    high: price + d("0.2"),
                    low: price - d("0.2"),
                    close: price,
                    volume: d("10000"),
                    closed: true,
                }
            })
            .collect()
    }
    #[test]
    fn candle_close_signal_only_fills_next_bar() {
        let cs = candles();
        let r = replay_candles(&cs, &meta(), &request()).unwrap();
        let fills = r["fills"].as_array().unwrap();
        assert!(!fills.is_empty(), "{r}");
        assert!(fills[0]["ts"].as_i64().unwrap() >= cs[4].ts);
        assert!(r["net_pnl"].as_str().unwrap().parse::<D>().unwrap() < zero());
    }
    #[test]
    fn dataset_gap_is_rejected() {
        let mut cs = candles();
        cs[4].ts += 60000;
        assert!(replay_candles(&cs, &meta(), &request()).is_err());
    }
    #[test]
    fn insufficient_history_is_rejected() {
        let cs = candles();
        assert!(replay_candles(&cs[..3], &meta(), &request()).is_err());
    }
    #[test]
    fn replay_is_reproducible_and_separate_mode() {
        let cs = candles();
        let a = replay_candles(&cs, &meta(), &request()).unwrap();
        assert_eq!(a, replay_candles(&cs, &meta(), &request()).unwrap());
        assert_eq!(a["mode"], "REPLAY");
    }
    #[test]
    fn stop_wins_when_low_and_high_both_hit() {
        let mut cs = candles();
        cs[4].low = d("100");
        cs[4].high = d("120");
        let r = replay_candles(&cs, &meta(), &request()).unwrap();
        let fills = r["fills"].as_array().unwrap();
        assert!(fills.len() >= 2, "{r}");
        assert_eq!(fills[1]["side"], "SELL");
        assert!(fills[1]["price"].as_str().unwrap().parse::<D>().unwrap() < d("101"));
    }
    fn zero() -> D {
        D::ZERO
    }
}

async fn evaluate(State(a): State<App>, Json(r): Json<ReplayRequest>) -> Response {
    response(async{let _permit=a.replays.clone().try_acquire_owned().map_err(|_|anyhow::anyhow!("Replay/historie is bezig"))?;ensure!(r.spread_bps>=d("1")&&r.spread_bps<=d("30")&&r.slippage_bps>=D::ZERO&&r.slippage_bps<=d("20"),"Kostenparameters buiten bereik");let q=quotes(&a).await.remove(&r.instrument).ok_or_else(||anyhow::anyhow!("Geen verse EUR-metadata"))?;let cs=crate::history::dataset(&a,&r.instrument,&r.bar).await?;ensure!(cs.len()>=240,"Minimaal 240 gesloten candles vereist");let candidates=[(5,21),(9,21),(13,21),(5,34),(9,34),(13,34)];let summarize=|v:&Value|json!({"parameters":v["parameters"],"net_pnl":v["net_pnl"],"fees":v["fees"],"max_drawdown":v["max_drawdown"],"turnover":v["turnover"],"fill_count":v["fill_count"],"max_exposure":v["max_exposure"],"first_ms":v["first_ms"],"last_ms":v["last_ms"]});let mut sensitivity=vec![];let mut folds=vec![];let initial=cs.len()/2;let width=(cs.len()-initial)/2;
for fold in 0..2{let split=initial+fold*width;let end=if fold==1{cs.len()}else{split+width};let mut best:Option<(D,ReplayRequest)>=None;let mut training=vec![];for (fast,slow) in candidates{let mut config=r.clone();config.fast=fast;config.slow=slow;let report=replay_candles(&cs[..split],&q,&config)?;let score=report["net_pnl"].as_str().unwrap_or("0").parse::<D>()?;training.push(summarize(&report));if best.as_ref().is_none_or(|(old,_)|score>*old){best=Some((score,config));}}let (_,config)=best.unwrap();let test=replay_candles(&cs[split..end],&q,&config)?;folds.push(json!({"fold":fold+1,"training_bars":split,"test_bars":end-split,"selected_without_test_data":config.fast.to_string()+"/"+&config.slow.to_string(),"out_of_sample":summarize(&test),"curve":test["curve"]}));if fold==0{sensitivity=training;}}
let report=json!({"mode":"WALK_FORWARD","instrument":r.instrument,"bar":r.bar,"bars":cs.len(),"sensitivity_training_only":sensitivity,"folds":folds,"policy":"Expanding training window; parameters chosen only by training net P&L; two disjoint test windows; cold start per test window; same fixed risk/OMS and OHLC cost model","jev_comparison":{"state":"FORWARD_DECISIONS_REQUIRED","reason":"No retrospective model calls. Historic training knowledge may leak; compare actual recorded forward-paper deployments with matching strategy and limits."},"uncertainty":"Short recent candle sample; parameter selection overfits; no tick queue, profitability evidence or validated predictive model."});let id:i64=sqlx::query_scalar("INSERT INTO paper_v2.replays(report) VALUES($1) RETURNING id").bind(&report).fetch_one(&a.db).await?;Ok(json!({"id":id,"report":report}))}.await)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PineReplayRequest {
    source: String,
    inputs: Option<Value>,
    config: ReplayRequest,
}
async fn pine_replay(State(a): State<App>, Json(r): Json<PineReplayRequest>) -> Response {
    response(async {
        let _permit=a.replays.clone().try_acquire_owned().map_err(|_|anyhow::anyhow!("Replay/historie is bezig"))?;
        let meta=quotes(&a).await.remove(&r.config.instrument).ok_or_else(||anyhow::anyhow!("EUR-instrumentmetadata ontbreekt"))?;
        let mut cs=crate::history::dataset(&a,&r.config.instrument,&r.config.bar).await?;
        if cs.len()>5000{cs.drain(..cs.len()-5000);}
        let inputs=r.inputs.unwrap_or(Value::Null);
        let result=crate::studio::pine_on_candles(&a,&r.source,&r.config.instrument,&r.config.bar,&inputs,&cs).await?;
        ensure!(result["ok"]==true,"Pine compile/runtimefout: {}",result["diagnostics"]);
        let mut signals=BTreeMap::new();
        for i in result["intents"].as_array().ok_or_else(||anyhow::anyhow!("Pine-uitvoer ongeldig"))? {
            let ts=i["bar_ms"].as_i64().ok_or_else(||anyhow::anyhow!("Pine-tijd ongeldig"))?;
            if i["action"]=="CONSIDER_EXIT" {signals.insert(ts,"SELL".to_string());}
            else {signals.entry(ts).or_insert_with(||"BUY".into());}
        }
        let mut report=replay_signals(&cs,&meta,&r.config,Some(&signals))?;
        report["mode"]=json!("PINE_REPLAY");report["strategy"]=json!("pine-closed-bar-profile-1");
        report["pine"]=json!({"source":r.source,"inputs":inputs,"runtime":result["runtime"],"duration_ms":result["duration_ms"],"semantics":"Same isolated closed-bar runtime and fixed risk core as paper. Proposals execute only after the signal candle closes; native strategy.* unsupported."});
        let id:i64=sqlx::query_scalar("INSERT INTO paper_v2.replays(report) VALUES($1) RETURNING id").bind(&report).fetch_one(&a.db).await?;
        Ok(json!({"id":id,"report":report}))
    }.await)
}
