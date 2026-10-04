//! Versioned research documents and durable server alerts. No exchange capabilities.
use crate::{
    App, CandleQuery, candle_result,
    model::{interval, now_ms},
};
use anyhow::{Result, bail, ensure};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use std::time::Duration;

pub fn reply(r: Result<Value>) -> Response {
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
        .route("/api/studio/indicators", post(custom_indicators))
        .route("/api/studio/documents", get(documents).post(save))
        .route("/api/studio/pine", post(pine))
        .route("/api/studio/alerts", get(alerts).post(add_alert))
        .route("/api/studio/alerts/control", post(alert_control))
        .route("/api/studio/deliveries/read", post(read_alerts))
        .route("/api/studio/graph/validate", post(validate_graph))
}
#[derive(Deserialize)]
pub struct DocQuery {
    kind: String,
}
async fn documents(State(a): State<App>, Query(q): Query<DocQuery>) -> Response {
    reply(async{ensure!(["pine","graph","workspace","drawings"].contains(&q.kind.as_str()),"Documenttype ongeldig");let rows=sqlx::query("SELECT name,version,payload,created_ms FROM studio.documents WHERE kind=$1 ORDER BY created_ms DESC LIMIT 100").bind(q.kind).fetch_all(&a.db).await?;Ok(json!(rows.iter().map(|r|json!({"name":r.get::<String,_>("name"),"version":r.get::<i64,_>("version"),"payload":r.get::<Value,_>("payload"),"created_ms":r.get::<i64,_>("created_ms")})).collect::<Vec<_>>()))}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    kind: String,
    name: String,
    base_version: i64,
    payload: Value,
}
async fn save(State(a): State<App>, Json(r): Json<Save>) -> Response {
    reply(async{ensure!(["pine","graph","workspace","drawings"].contains(&r.kind.as_str()),"Documenttype ongeldig");ensure!(!r.name.trim().is_empty()&&r.name.len()<=80&&r.payload.to_string().len()<=14000,"Document te groot of naam ongeldig");if r.kind=="graph"{graph_plan(&r.payload)?;}if r.kind=="pine"{ensure!(r.payload["source"].as_str().is_some_and(|s|s.len()<=12000),"Pine-bron ontbreekt");}let mut tx=a.db.begin().await?;sqlx::query("SELECT pg_advisory_xact_lock(478003)").execute(&mut *tx).await?;let v:Option<i64>=sqlx::query_scalar("SELECT max(version) FROM studio.documents WHERE kind=$1 AND name=$2").bind(&r.kind).bind(&r.name).fetch_one(&mut *tx).await?;ensure!(v.unwrap_or(0)==r.base_version,"Een nieuwere versie bestaat; laad die eerst om overschrijven te voorkomen");let version=r.base_version+1;sqlx::query("INSERT INTO studio.documents(kind,name,version,payload,created_ms) VALUES($1,$2,$3,$4,$5)").bind(r.kind).bind(r.name).bind(version).bind(r.payload).bind(now_ms()).execute(&mut *tx).await?;tx.commit().await?;Ok(json!({"version":version}))}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PineRequest {
    source: String,
    inputs: Option<Value>,
    instrument: String,
    bar: String,
}
pub async fn run_pine(
    a: &App,
    source: &str,
    instrument: &str,
    bar: &str,
    inputs: &Value,
) -> Result<Value> {
    let data = candle_result(
        a,
        &CandleQuery {
            instrument: instrument.into(),
            bar: bar.into(),
            after: None,
        },
    )
    .await?;
    ensure!(data["gaps"] == 0, "Datagat in Pine-historie");
    let step = interval(bar).ok_or_else(|| anyhow::anyhow!("Timeframe ongeldig"))?;
    let candles: Vec<_> = data["candles"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["closed"] == true && c["ts"].as_i64().is_some_and(|t| t + step <= now_ms()))
        .cloned()
        .collect();
    let cs: Vec<crate::model::Candle> = serde_json::from_value(json!(candles))?;
    pine_on_candles(a, source, instrument, bar, inputs, &cs).await
}
pub async fn pine_on_candles(
    a: &App,
    source: &str,
    instrument: &str,
    bar: &str,
    inputs: &Value,
    candles: &[crate::model::Candle],
) -> Result<Value> {
    let _permit = a
        .scripts
        .clone()
        .try_acquire_owned()
        .map_err(|_| anyhow::anyhow!("Er draait al een Pine-script; probeer later opnieuw"))?;
    ensure!(source.len() <= 12000, "Maximaal 12 KB Pine-bron");
    ensure!(
        inputs.is_null() || (inputs.is_object() && inputs.to_string().len() <= 2000),
        "Pine-inputs ongeldig of te groot"
    );
    let step = interval(bar).ok_or_else(|| anyhow::anyhow!("Timeframe ongeldig"))?;
    use ring::rand::SecureRandom;
    let mut nonce = [0u8; 16];
    ring::rand::SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| anyhow::anyhow!("Entropie niet beschikbaar"))?;
    let id: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let path = format!("/jobs/{id}.job");
    let tmp = format!("{path}.tmp");
    let result_path = format!("{path}.result");
    let input = json!({"source":source,"inputs":inputs,"instrument":instrument,"timeframe":bar,"step":step,"candles":candles});
    tokio::fs::write(&tmp, serde_json::to_vec(&input)?).await?;
    tokio::fs::rename(&tmp, &path).await?;
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(bytes) = tokio::fs::read(&result_path).await {
                ensure!(bytes.len() <= 1_000_000, "Pine-uitvoer te groot");
                let v: Value = serde_json::from_slice(&bytes)?;
                return Ok::<_, anyhow::Error>(v);
            }
            tokio::time::sleep(Duration::from_millis(75)).await;
        }
    })
    .await;
    tokio::fs::remove_file(&path).await.ok();
    tokio::fs::remove_file(&result_path).await.ok();
    result.map_err(|_| anyhow::anyhow!("Pine-worker niet beschikbaar of deadline verstreken"))?
}
async fn pine(State(a): State<App>, Json(r): Json<PineRequest>) -> Response {
    reply(
        run_pine(
            &a,
            &r.source,
            &r.instrument,
            &r.bar,
            &r.inputs.unwrap_or(Value::Null),
        )
        .await,
    )
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Alert {
    pub name: String,
    pub instrument: String,
    pub bar: String,
    pub field: String,
    pub comparison: String,
    pub value: f64,
    pub trigger: String,
    pub cooldown_secs: i64,
    pub bot_id: Option<String>,
    pub drawing: Option<DrawingLine>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DrawingLine {
    pub first_ms: i64,
    pub second_ms: i64,
    pub first_price: f64,
    pub second_price: f64,
}
impl DrawingLine {
    fn price_at(&self, ts: i64) -> Result<f64> {
        ensure!(
            self.first_ms > 0
                && self.second_ms > self.first_ms
                && self.first_price.is_finite()
                && self.second_price.is_finite()
                && self.first_price > 0.
                && self.second_price > 0.,
            "Tekening vereist twee oplopende tijden en positieve eindige prijzen"
        );
        let p = self.first_price
            + (self.second_price - self.first_price) * (ts - self.first_ms) as f64
                / (self.second_ms - self.first_ms) as f64;
        ensure!(p.is_finite() && p > 0., "Geprojecteerde lijnprijs ongeldig");
        Ok(p)
    }
}
fn check_alert(r: &Alert) -> Result<()> {
    ensure!(
        !r.name.trim().is_empty() && r.name.len() <= 80,
        "Alertnaam ongeldig"
    );
    ensure!(interval(&r.bar).is_some(), "Timeframe ongeldig");
    ensure!(
        !r.field.starts_with("bot_")
            || r.bot_id
                .as_ref()
                .is_some_and(|s| !s.is_empty() && s.len() <= 32),
        "Bot-id vereist"
    );
    ensure!(
        [
            "price",
            "drawing_line",
            "sma",
            "ema",
            "rsi",
            "macd",
            "atr",
            "bb_upper",
            "bb_lower",
            "vwap",
            "stochastic",
            "adx",
            "obv",
            "risk_paused",
            "bot_running",
            "bot_drawdown",
            "bot_total_loss"
        ]
        .contains(&r.field.as_str()),
        "Alertveld niet ondersteund"
    );
    ensure!(
        ["above", "below"].contains(&r.comparison.as_str()) && r.value.is_finite(),
        "Drempel ongeldig"
    );
    ensure!(
        ["intrabar", "close"].contains(&r.trigger.as_str())
            && (10..=86400).contains(&r.cooldown_secs),
        "Trigger of cooldown ongeldig"
    );
    ensure!(
        r.trigger == "close"
            || r.field == "price"
            || r.field == "drawing_line"
            || r.field == "risk_paused"
            || r.field.starts_with("bot_"),
        "Indicatoralerts vereisen candle-close"
    );
    if r.field == "drawing_line" {
        r.drawing
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tekening ontbreekt"))?
            .price_at(now_ms())?;
        ensure!(
            r.value == 0.,
            "Lijnalert vergelijkt koers met de lijn (drempel 0)"
        );
    }
    Ok(())
}
async fn add_alert(State(a): State<App>, Json(r): Json<Alert>) -> Response {
    reply(
        async {
            check_alert(&r)?;
            ensure!(
                a.market
                    .read()
                    .await
                    .instruments
                    .iter()
                    .any(|i| i["instId"] == r.instrument),
                "Instrument niet beschikbaar"
            );
            let mut tx = a.db.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(478004)")
                .execute(&mut *tx)
                .await?;
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM studio.alerts WHERE enabled")
                .fetch_one(&mut *tx)
                .await?;
            ensure!(count < 40, "Maximaal 40 actieve alerts");
            let id: i64 =
                sqlx::query_scalar("INSERT INTO studio.alerts(payload) VALUES($1) RETURNING id")
                    .bind(serde_json::to_value(r)?)
                    .fetch_one(&mut *tx)
                    .await?;
            tx.commit().await?;
            Ok(json!({"id":id}))
        }
        .await,
    )
}
async fn alerts(State(a): State<App>) -> Response {
    reply(async{let rules=sqlx::query("SELECT id,payload,enabled,last_sent,error FROM studio.alerts ORDER BY id DESC LIMIT 100").fetch_all(&a.db).await?;let events=sqlx::query("SELECT id,alert_id,ts,payload,read FROM studio.deliveries ORDER BY id DESC LIMIT 100").fetch_all(&a.db).await?;Ok(json!({"rules":rules.iter().map(|r|json!({"id":r.get::<i64,_>("id"),"config":r.get::<Value,_>("payload"),"enabled":r.get::<bool,_>("enabled"),"last_sent":r.get::<i64,_>("last_sent"),"error":r.get::<Option<String>,_>("error")})).collect::<Vec<_>>(),"events":events.iter().map(|r|json!({"id":r.get::<i64,_>("id"),"alert_id":r.get::<i64,_>("alert_id"),"ts":r.get::<i64,_>("ts"),"detail":r.get::<Value,_>("payload"),"read":r.get::<bool,_>("read")})).collect::<Vec<_>>()}))}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    id: i64,
    enabled: bool,
}
async fn alert_control(State(a): State<App>, Json(r): Json<Control>) -> Response {
    reply(
        async {
            let mut tx = a.db.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(478004)")
                .execute(&mut *tx)
                .await?;
            if r.enabled {
                let count: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM studio.alerts WHERE enabled AND id<>$1",
                )
                .bind(r.id)
                .fetch_one(&mut *tx)
                .await?;
                ensure!(count < 40, "Maximaal 40 actieve alerts");
            }
            ensure!(
                sqlx::query("UPDATE studio.alerts SET enabled=$1 WHERE id=$2")
                    .bind(r.enabled)
                    .bind(r.id)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected()
                    == 1,
                "Alert niet gevonden"
            );
            tx.commit().await?;
            Ok(json!({"ok":true}))
        }
        .await,
    )
}
async fn read_alerts(State(a): State<App>) -> Response {
    reply(
        async {
            sqlx::query("UPDATE studio.deliveries SET read=TRUE WHERE NOT read")
                .execute(&a.db)
                .await?;
            Ok(json!({"ok":true}))
        }
        .await,
    )
}
async fn alert_value(a: &App, r: &Alert) -> Result<(f64, i64)> {
    if r.field.starts_with("bot_") {
        let p: Value = sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1")
            .fetch_one(&a.db)
            .await?;
        let b = &p["bots"][r.bot_id.as_deref().unwrap_or("")];
        ensure!(b.is_object(), "Alertbot ontbreekt");
        let value = match r.field.as_str() {
            "bot_running" => {
                if b["running"] == true {
                    1.
                } else {
                    0.
                }
            }
            "bot_drawdown" => b["ledger"]["drawdown"].as_str().unwrap_or("").parse()?,
            _ => b["ledger"]["total_loss"].as_str().unwrap_or("").parse()?,
        };
        return Ok((value, now_ms()));
    }

    if r.field == "risk_paused" {
        let p: Option<Value> =
            sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1")
                .fetch_optional(&a.db)
                .await?;
        let p = p.ok_or_else(|| anyhow::anyhow!("Rekening nog niet ingesteld"))?;
        return Ok((
            if p["ledger"]["paused"] == true {
                1.0
            } else {
                0.0
            },
            now_ms(),
        ));
    }
    if r.trigger == "intrabar" {
        let m = a.market.read().await;
        ensure!(m.feed_connected, "Marktverbinding verbroken");
        let t = m
            .tickers
            .get(&r.instrument)
            .ok_or_else(|| anyhow::anyhow!("Ticker nog niet ontvangen"))?;
        let ts = t["ts"].as_str().unwrap_or("0").parse::<i64>()?;
        ensure!((0..15000).contains(&(now_ms() - ts)), "Koers niet vers");
        let value: f64 = t["last"].as_str().unwrap_or("").parse()?;
        return Ok((
            if r.field == "drawing_line" {
                value
                    - r.drawing
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Tekening ontbreekt"))?
                        .price_at(ts)?
            } else {
                value
            },
            ts,
        ));
    }
    let d = candle_result(
        a,
        &CandleQuery {
            instrument: r.instrument.clone(),
            bar: r.bar.clone(),
            after: None,
        },
    )
    .await?;
    ensure!(
        d["gaps"] == 0 && d["fresh"] == true,
        "Candledata niet betrouwbaar"
    );
    let c = d["candles"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|c| c["closed"] == true)
        .ok_or_else(|| anyhow::anyhow!("Geen gesloten candle"))?;
    let ts = c["ts"].as_i64().unwrap();
    let v = if r.field == "price" || r.field == "drawing_line" {
        c["close"].as_str().unwrap_or("").parse()?
    } else {
        d["indicators"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["ts"] == ts)
            .and_then(|i| i[&r.field].as_f64())
            .ok_or_else(|| anyhow::anyhow!("Indicator warm-up ontbreekt"))?
    };
    Ok((
        if r.field == "drawing_line" {
            v - r
                .drawing
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Tekening ontbreekt"))?
                .price_at(ts + interval(&r.bar).unwrap())?
        } else {
            v
        },
        ts,
    ))
}
pub async fn run(a: App) {
    loop {
        if let Err(e) = alert_cycle(&a).await {
            tracing::warn!(error=%e,"Serveralerts uitgesteld")
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
async fn alert_cycle(a: &App) -> Result<()> {
    let rows =
        sqlx::query("SELECT id,payload FROM studio.alerts WHERE enabled ORDER BY id LIMIT 40")
            .fetch_all(&a.db)
            .await?;
    for row in rows {
        let id: i64 = row.get("id");
        let r: Alert = serde_json::from_value(row.get("payload"))?;
        let value = alert_value(a, &r).await;
        match value {
            Err(e) => {
                sqlx::query("UPDATE studio.alerts SET error=$1 WHERE id=$2")
                    .bind(e.to_string())
                    .bind(id)
                    .execute(&a.db)
                    .await?;
            }
            Ok((v, bar)) => {
                let mut tx = a.db.begin().await?;
                let s=sqlx::query("SELECT enabled,last_condition,last_bar,last_sent FROM studio.alerts WHERE id=$1 FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
                if !s.get::<bool, _>("enabled") {
                    continue;
                }
                let condition = if r.comparison == "above" {
                    v > r.value
                } else {
                    v < r.value
                };
                let now = now_ms();
                let duplicate = r.trigger == "close" && bar <= s.get::<i64, _>("last_bar");
                let fire = !duplicate
                    && condition
                    && !s.get::<bool, _>("last_condition")
                    && now - s.get::<i64, _>("last_sent") >= r.cooldown_secs * 1000;
                if fire {
                    sqlx::query("INSERT INTO studio.deliveries(alert_id,ts,payload) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(id).bind(now).bind(json!({"name":r.name,"instrument":r.instrument,"field":r.field,"value":v,"bar_ms":bar,"trigger":r.trigger,"delivery":"LOCAL_INBOX","authority":"NOTIFICATION_ONLY"})).execute(&mut *tx).await?;
                }
                if !duplicate {
                    sqlx::query("UPDATE studio.alerts SET last_condition=$1,last_bar=$2,last_sent=CASE WHEN $3 THEN $4 ELSE last_sent END,error=NULL WHERE id=$5").bind(condition).bind(bar).bind(fire).bind(now).bind(id).execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
        }
    }
    Ok(())
}
// Graph is a declarative plan. No script or UI node can remove the risk gate.
pub fn graph_plan(v: &Value) -> Result<Vec<String>> {
    ensure!(v["schema"] == 1, "Strategie vereist schema 1");
    let ns = v["nodes"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Nodes ontbreken"))?;
    let es = v["edges"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Verbindingen ontbreken"))?;
    ensure!(
        (3..=32).contains(&ns.len()) && es.len() <= 64,
        "Grafiekgrootte buiten limiet"
    );
    let mut degree = std::collections::BTreeMap::new();
    let mut kinds = std::collections::BTreeMap::new();
    for n in ns {
        let id = n["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Node-id ontbreekt"))?;
        ensure!(
            id.len() <= 40 && degree.insert(id, 0usize).is_none(),
            "Dubbele/ongeldige node-id"
        );
        let kind = n["kind"].as_str().unwrap_or("");
        ensure!(
            [
                "market",
                "sma_cross",
                "rsi_range",
                "condition",
                "and",
                "or",
                "time_window",
                "cooldown",
                "news",
                "jev",
                "pine",
                "sizing",
                "intent",
                "risk_gate"
            ]
            .contains(&kind),
            "Onbekend nodetype: {kind}"
        );
        kinds.insert(id, kind);
    }
    let mut adjacency = std::collections::BTreeMap::<&str, Vec<&str>>::new();
    let mut pairs = std::collections::BTreeSet::new();
    for e in es {
        let from = e["source"].as_str().unwrap_or("");
        let to = e["target"].as_str().unwrap_or("");
        ensure!(
            degree.contains_key(from) && degree.contains_key(to) && pairs.insert((from, to)),
            "Ongeldige of dubbele verbinding"
        );
        *degree.get_mut(to).unwrap() += 1;
        adjacency.entry(from).or_default().push(to);
    }
    ensure!(
        kinds.values().filter(|k| **k == "risk_gate").count() == 1
            && kinds.values().filter(|k| **k == "intent").count() == 1
            && kinds.values().filter(|k| **k == "market").count() == 1,
        "Eén marktbron, intent en vaste risicokern vereist"
    );
    let gate = *kinds.iter().find(|(_, k)| **k == "risk_gate").unwrap().0;
    let intent = *kinds.iter().find(|(_, k)| **k == "intent").unwrap().0;
    ensure!(
        adjacency.get(intent).is_some_and(|a| a == &vec![gate])
            && !adjacency.contains_key(gate)
            && degree[gate] == 1,
        "Intent mag uitsluitend naar vaste risicokern"
    );
    for id in kinds.keys() {
        ensure!(
            *id == gate || adjacency.get(id).is_some_and(|v| !v.is_empty()),
            "Doodlopende node; alle routes moeten bij risk gate eindigen"
        );
    }
    let mut order = vec![];
    while order.len() < ns.len() {
        let next = degree
            .iter()
            .find(|(id, d)| **d == 0 && !order.contains(&id.to_string()))
            .map(|(id, _)| *id);
        let Some(id) = next else {
            bail!("Cyclische grafiek; onbegrensde lussen zijn verboden")
        };
        if order.is_empty() {
            ensure!(
                kinds[id] == "market",
                "Alle nodes moeten vanaf de marktbron verbonden zijn"
            );
        } else {
            ensure!(
                degree[id] == 0 && es.iter().any(|e| e["target"] == id),
                "Losse node verboden"
            );
        }
        order.push(id.to_string());
        if let Some(children) = adjacency.get(id) {
            for c in children {
                *degree.get_mut(c).unwrap() -= 1;
            }
        }
    }
    ensure!(
        order.last().is_some_and(|id| id == gate),
        "Alle takken moeten eindigen bij de risicokern"
    );
    Ok(order)
}
async fn validate_graph(Json(v): Json<Value>) -> Response {
    reply(graph_plan(&v).and_then(|order|{crate::graph::validate_parameters(&v)?;Ok(order)}).map(|order|json!({"valid":true,"plan":order,"authority":"PROPOSALS_ONLY","risk_gate":"MANDATORY"})))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> Value {
        json!({"schema":1,"nodes":[{"id":"m","kind":"market"},{"id":"i","kind":"intent"},{"id":"r","kind":"risk_gate"}],"edges":[{"source":"m","target":"i"},{"source":"i","target":"r"}]})
    }
    #[test]
    fn drawing_alert_projects_time_not_screen_pixels() {
        let d = DrawingLine {
            first_ms: 1000,
            second_ms: 2000,
            first_price: 100.,
            second_price: 110.,
        };
        assert_eq!(d.price_at(3000).unwrap(), 120.);
        assert!(
            DrawingLine {
                second_ms: 1000,
                ..d
            }
            .price_at(3000)
            .is_err()
        );
    }
    #[test]
    fn graph_gate_required() {
        let mut v = graph();
        assert!(graph_plan(&v).is_ok());
        v["nodes"][2]["kind"] = json!("sizing");
        assert!(graph_plan(&v).is_err());
    }
    #[test]
    fn cycles_rejected() {
        let mut v = graph();
        v["edges"]
            .as_array_mut()
            .unwrap()
            .push(json!({"source":"i","target":"m"}));
        assert!(graph_plan(&v).is_err());
    }
    #[test]
    fn alert_intrabar_indicators_rejected() {
        let mut a = Alert {
            name: "test".into(),
            bot_id: None,
            drawing: None,
            instrument: "BTC-EUR".into(),
            bar: "1m".into(),
            field: "rsi".into(),
            comparison: "above".into(),
            value: 50.,
            trigger: "intrabar".into(),
            cooldown_secs: 30,
        };
        assert!(check_alert(&a).is_err());
        a.trigger = "close".into();
        assert!(check_alert(&a).is_ok());
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IndicatorRequest {
    instrument: String,
    bar: String,
    settings: crate::indicators::Settings,
}
async fn custom_indicators(State(a): State<App>, Json(r): Json<IndicatorRequest>) -> Response {
    reply(
        async {
            r.settings.validate()?;
            let mut d = candle_result(
                &a,
                &CandleQuery {
                    instrument: r.instrument,
                    bar: r.bar.clone(),
                    after: None,
                },
            )
            .await?;
            let cs: Vec<crate::model::Candle> = serde_json::from_value(d["candles"].clone())?;
            d["indicators"] = serde_json::to_value(crate::indicators::calculate_with(
                &cs,
                interval(&r.bar).unwrap(),
                &r.settings,
            ))?;
            d["indicator_parameters"] = serde_json::to_value(r.settings)?;
            Ok(d)
        }
        .await,
    )
}
