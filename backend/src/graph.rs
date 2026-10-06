//! Immutable graph deployments and bounded proposals for the existing fixed risk core.
use crate::{
    App,
    model::now_ms,
    paper,
    risk::{Bot, Paper, sma_signal, strategy_intent},
    studio::{graph_plan, reply},
};
use anyhow::{Result, bail, ensure};
use axum::{
    Json, Router,
    extract::State,
    response::Response,
    routing::{get, post},
};
use rust_decimal::Decimal as D;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub fn router() -> Router<App> {
    Router::new()
        .route("/api/studio/deploy", post(deploy))
        .route("/api/studio/install", post(install))
        .route("/api/studio/system-bots", get(system_bots))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Deploy {
    bot_id: String,
    name: String,
    version: i64,
}
async fn deploy(State(a): State<App>, Json(r): Json<Deploy>) -> Response {
    reply(async{ensure!(r.bot_id!="manual","Handmatig budget is geen strategie-deployment");let v:Value=sqlx::query_scalar("SELECT payload FROM studio.documents WHERE kind='graph' AND name=$1 AND version=$2").bind(&r.name).bind(r.version).fetch_one(&a.db).await?;graph_plan(&v)?;validate_parameters(&v)?;ensure!(v["bot"].is_null(),"Gebruik Toepassen vanuit Bot Studio voor een bot met opgeslagen instellingen");let mut tx=a.db.begin().await?;let mut p=paper::load(&mut tx).await?;ensure!(!p.positions.values().any(|x|x.bot_id==r.bot_id&&x.quantity>D::ZERO)&&!p.orders.values().any(|x|x.intent.bot_id==r.bot_id&&x.state.open()),"Sluit exposure en orders voordat een andere versie wordt gekoppeld");let b=p.bots.get_mut(&r.bot_id).ok_or_else(||anyhow::anyhow!("Bot bestaat niet"))?;ensure!(!b.running,"Pauzeer de bot vóór een versie-wijziging");b.version=format!("graph:{}:{}",r.name,r.version);b.last_signal="Nieuwe versie gekoppeld; wacht op expliciete start".into();b.last_bar=(now_ms()/60000-1)*60000;sqlx::query("INSERT INTO studio.deployments(bot_id,name,version,created_ms) VALUES($1,$2,$3,$4) ON CONFLICT(bot_id) DO UPDATE SET name=$2,version=$3,created_ms=$4").bind(&r.bot_id).bind(&r.name).bind(r.version).bind(now_ms()).execute(&mut *tx).await?;p.event(now_ms(),"DEPLOY_GRAPH",&r.bot_id,&format!("{} v{}",r.name,r.version));paper::persist(&mut tx,&mut p).await?;tx.commit().await?;Ok(json!({"ok":true}))}.await)
}
fn int(p: &Value, k: &str, default: i64, min: i64, max: i64) -> Result<i64> {
    let n = if p[k].is_null() {
        default
    } else {
        p[k].as_i64()
            .ok_or_else(|| anyhow::anyhow!("Parameter {k} vereist geheel getal"))?
    };
    ensure!(
        (min..=max).contains(&n),
        "Parameter {k} buiten bereik {min}–{max}"
    );
    Ok(n)
}
pub(crate) fn validate_parameters(v: &Value) -> Result<()> {
    if !v["bot"].is_null() {
        ensure!(
            ["paper", "live"].contains(&v["bot"]["mode"].as_str().unwrap_or("")),
            "Kies Paper of Live"
        );
        ensure!(["1m", "5m", "15m"].contains(&v["bot"]["bar"].as_str().unwrap_or("1m")), "Botinterval ongeldig");
        let config: paper::BotConfig = serde_json::from_value(v["bot"]["config"].clone())?;
        let mut check = Paper::new(config.limits.clone(), config.limits.clone(), now_ms())?;
        check.bots.clear();
        check.add_bot(config.bot(now_ms())?, now_ms())?;
    }
    ensure!(
        v["bot"]["ai_enabled"].is_null() || v["bot"]["ai_enabled"].is_boolean(),
        "AI-keuze ongeldig"
    );
    ensure!(
        v["bot"]["ai_enabled"] == true
            || !v["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["kind"] == "jev"),
        "AI staat uit. Verwijder de Jev-node of schakel AI bewust in bij Instellingen"
    );
    for n in v["nodes"].as_array().unwrap() {
        let p = &n["params"];
        match n["kind"].as_str().unwrap_or("") {
            "sma_cross" | "ema_cross" | "macd_cross" => {
                ensure!(
                    int(p, "fast", 9, 2, 99)? < int(p, "slow", 21, 3, 100)?,
                    "SMA fast moet kleiner dan slow zijn"
                );
            }
            "bollinger" => {
                int(p, "period", 20, 2, 100)?;
                number(p, "deviations", 2., 0.1, 5.)?;
            }
            "candle_stop" => {
                number(p, "drop_pct", 1., 0.01, 50.)?;
                ensure!(
                    ["close_pause", "close", "pause"]
                        .contains(&p["action"].as_str().unwrap_or("close_pause")),
                    "Candle-stopactie ongeldig"
                );
            }
            "rsi_range" => {
                ensure!(
                    int(p, "low", 30, 1, 49)? < int(p, "high", 70, 51, 99)?,
                    "RSI-grenzen ongeldig"
                );
            }
            "time_window" => {
                int(p, "start_hour", 0, 0, 23)?;
                int(p, "end_hour", 23, 0, 23)?;
            }
            "cooldown" => {
                int(p, "bars", 5, 1, 1440)?;
            }
            "condition" | "if_else" => {
                ensure!(
                    ["rsi", "adx", "atr", "sma", "ema"]
                        .contains(&p["field"].as_str().unwrap_or("rsi")),
                    "Onbekend indicatorveld"
                );
                ensure!(
                    ["above", "below"].contains(&p["comparison"].as_str().unwrap_or("above")),
                    "Vergelijking ongeldig"
                );
                ensure!(
                    p["value"].is_null() || p["value"].as_f64().is_some_and(|v| v.is_finite()),
                    "Drempel ongeldig"
                );
            }
            "pine" => {
                ensure!(
                    p["name"].as_str().is_some() && p["version"].as_i64().is_some_and(|v| v > 0),
                    "Pine-node vereist opgeslagen naam en exacte versie"
                );
            }
            "action" => {
                ensure!(
                    ["buy", "sell", "hold"].contains(&p["action"].as_str().unwrap_or("hold")),
                    "Actie ongeldig"
                );
            }
            "news" => {
                int(p, "quiet_minutes", 30, 5, 1440)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn number(p: &Value, k: &str, default: f64, min: f64, max: f64) -> Result<f64> {
    let n = if p[k].is_null() {
        default
    } else {
        p[k].as_f64()
            .ok_or_else(|| anyhow::anyhow!("Parameter {k} vereist een getal"))?
    };
    ensure!(
        n.is_finite() && n >= min && n <= max,
        "Parameter {k} buiten bereik {min}–{max}"
    );
    Ok(n)
}
// These guards are unconditional safety checks, independent of inactive signal branches.
pub(crate) fn candle_stop(v: &Value, features: &Value) -> Result<Option<String>> {
    let mut close = false;
    let mut pause = false;
    let mut hit = false;
    for n in v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["kind"] == "candle_stop")
    {
        let p = &n["params"];
        let open: D = features["candle_open"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Candle-open ontbreekt"))?
            .parse()?;
        let end: D = features["candle_close"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Candle-close ontbreekt"))?
            .parse()?;
        ensure!(open > D::ZERO && end > D::ZERO, "Candleprijzen ongeldig");
        let threshold: D = number(p, "drop_pct", 1., 0.01, 50.)?.to_string().parse()?;
        if (open - end) * D::from(100) >= open * threshold {
            hit = true;
            match p["action"].as_str().unwrap_or("close_pause") {
                "close" => close = true,
                "pause" => pause = true,
                _ => {
                    close = true;
                    pause = true;
                }
            }
        }
    }
    Ok(if !hit {
        None
    } else {
        Some(
            if close && pause {
                "STOP_CLOSE"
            } else if close {
                "EXIT_NOW"
            } else {
                "STOP_PAUSE"
            }
            .into(),
        )
    })
}
fn ema(values: &[D], period: usize) -> Vec<D> {
    if values.len() < period {
        return vec![];
    }
    let mut value = values[..period].iter().sum::<D>() / D::from(period);
    let mut out = vec![value];
    let alpha = D::from(2) / D::from(period + 1);
    for price in &values[period..] {
        value += alpha * (*price - value);
        out.push(value);
    }
    out
}
fn cross(a0: D, a1: D, b0: D, b1: D) -> i8 {
    if a0 <= b0 && a1 > b1 {
        1
    } else if a0 >= b0 && a1 < b1 {
        -1
    } else {
        0
    }
}
fn standard_signal(kind: &str, p: &Value, closes: &[D]) -> Result<i8> {
    if kind == "bollinger" {
        use rust_decimal::prelude::ToPrimitive;
        let period = int(p, "period", 20, 2, 100)? as usize;
        if closes.len() < period {
            return Ok(0);
        }
        let xs: Vec<f64> = closes[closes.len() - period..]
            .iter()
            .map(|v| v.to_f64().unwrap())
            .collect();
        let mean = xs.iter().sum::<f64>() / period as f64;
        let deviation = (xs.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / period as f64).sqrt();
        let price = *xs.last().unwrap();
        let multiple = number(p, "deviations", 2., 0.1, 5.)?;
        return Ok(if price < mean - multiple * deviation {
            1
        } else if price >= mean {
            -1
        } else {
            0
        });
    }
    let fast = int(p, "fast", if kind == "macd_cross" { 12 } else { 9 }, 2, 99)? as usize;
    let slow = int(
        p,
        "slow",
        if kind == "macd_cross" { 26 } else { 21 },
        3,
        100,
    )? as usize;
    ensure!(fast < slow, "Fast moet kleiner dan slow zijn");
    if closes.len() < slow + 1 {
        return Ok(0);
    }
    let a = ema(closes, fast);
    let b = ema(closes, slow);
    if kind == "ema_cross" {
        return Ok(cross(
            a[a.len() - 2],
            a[a.len() - 1],
            b[b.len() - 2],
            b[b.len() - 1],
        ));
    }
    let macd: Vec<D> = b
        .iter()
        .enumerate()
        .map(|(i, v)| a[i + slow - fast] - *v)
        .collect();
    let signal = ema(&macd, 9);
    if signal.len() < 2 {
        return Ok(0);
    }
    Ok(cross(
        macd[macd.len() - 2],
        macd[macd.len() - 1],
        signal[signal.len() - 2],
        signal[signal.len() - 1],
    ))
}
async fn system_bots() -> Json<Value> {
    Json(
        serde_json::from_str(include_str!("../system_bots.json"))
            .expect("validated system bot catalog"),
    )
}

// A missing branch is distinct from HOLD. NOT and OR cannot revive an inactive branch.
fn parents(
    v: &Value,
    id: &str,
    values: &BTreeMap<String, i8>,
    branches: &BTreeMap<String, bool>,
) -> Vec<i8> {
    v["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["target"] == id)
        .filter_map(|e| {
            let from = e["source"].as_str()?;
            if let Some(taken) = branches.get(from) {
                if e["sourceHandle"].as_str() != Some(if *taken { "true" } else { "false" }) {
                    return None;
                }
            }
            values.get(from).copied()
        })
        .collect()
}
fn incoming(parents: &[i8]) -> i8 {
    if parents.contains(&-1) {
        -1
    } else if !parents.is_empty() && parents.iter().all(|x| *x > 0) {
        1
    } else {
        0
    }
}
fn condition(p: &Value, features: &Value) -> Result<bool> {
    let field = p["field"].as_str().unwrap_or("rsi");
    let val = features[field]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("Indicator {field}: warm-up ontbreekt"))?;
    let threshold = p["value"].as_f64().unwrap_or(50.);
    Ok(if p["comparison"] == "below" {
        val < threshold
    } else {
        val > threshold
    })
}
fn node_value(
    kind: &str,
    p: &Value,
    ps: &[i8],
    closes: &[D],
    features: &Value,
    ts: i64,
    last_order: i64,
) -> Result<i8> {
    let i = incoming(ps);
    Ok(match kind {
        "market" => 1,
        "sma_cross" if i != 0 => match sma_signal(
            closes,
            int(p, "fast", 9, 2, 99)? as usize,
            int(p, "slow", 21, 3, 100)? as usize,
        ) {
            Some("BUY") => 1,
            Some("SELL") => -1,
            _ => 0,
        },
        "rsi_range" if i != 0 => {
            let r = features["rsi"]
                .as_f64()
                .ok_or_else(|| anyhow::anyhow!("RSI warm-up ontbreekt"))?;
            if r < (int(p, "low", 30, 1, 49)? as f64) {
                1
            } else if r > (int(p, "high", 70, 51, 99)? as f64) {
                -1
            } else {
                0
            }
        }
        "ema_cross" | "macd_cross" | "bollinger" if i != 0 => standard_signal(kind, p, closes)?,
        "sma_cross" | "rsi_range" | "ema_cross" | "macd_cross" | "bollinger" => 0,
        "condition" => {
            if i < 0 || condition(p, features)? {
                i
            } else {
                0
            }
        }
        "if_else" | "and" | "sizing" | "intent" | "risk_gate" | "candle_stop" => i,
        "or" => {
            if ps.contains(&-1) {
                -1
            } else if ps.contains(&1) {
                1
            } else {
                0
            }
        }
        "not" => {
            if i < 0 {
                -1
            } else if i == 0 {
                1
            } else {
                0
            }
        }
        "action" => {
            if i == 0 {
                0
            } else {
                match p["action"].as_str().unwrap_or("hold") {
                    "buy" => 1,
                    "sell" => -1,
                    _ => 0,
                }
            }
        }
        "time_window" => {
            let h = (ts / 3600000) % 24;
            let start = int(p, "start_hour", 0, 0, 23)?;
            let end = int(p, "end_hour", 23, 0, 23)?;
            let active = if start <= end {
                h >= start && h <= end
            } else {
                h >= start || h <= end
            };
            if i < 0 || active { i } else { 0 }
        }
        "cooldown" => {
            if i < 0 || ts + 60000 - last_order >= int(p, "bars", 5, 1, 1440)? * 60000 {
                i
            } else {
                0
            }
        }
        _ => bail!("Node vereist externe data: {kind}"),
    })
}
pub(crate) fn pine_value(result: &Value, p: &Value, ts: i64) -> Result<i8> {
    if let Some(title) = p["plot"].as_str().filter(|s| !s.is_empty()) {
        let plot = result["plots"]
            .as_array()
            .and_then(|ps| ps.iter().find(|x| x["title"] == title))
            .ok_or_else(|| anyhow::anyhow!("Pine-plot {title} ontbreekt"))?;
        let val = plot["data"]
            .as_array()
            .and_then(|ds| ds.iter().find(|x| x["time"] == ts))
            .and_then(|x| x["value"].as_f64());
        return Ok(
            if val.is_some_and(|v| {
                if p["comparison"] == "below" {
                    v < p["value"].as_f64().unwrap_or(0.)
                } else {
                    v > p["value"].as_f64().unwrap_or(0.)
                }
            }) {
                1
            } else {
                0
            },
        );
    }
    let mut direction = 0;
    for i in result["intents"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Pine-uitvoer ongeldig"))?
    {
        if i["bar_ms"] == ts {
            if i["action"] == "CONSIDER_EXIT" {
                direction = -1
            } else if direction != -1 {
                direction = 1
            }
        }
    }
    Ok(direction)
}
pub(crate) fn evaluate(
    v: &Value,
    plan: &[String],
    closes: &[D],
    features: &Value,
    ts: i64,
    last_order: i64,
    external: &BTreeMap<String, i8>,
) -> Result<String> {
    if let Some(stop) = candle_stop(v, features)? {
        return Ok(stop);
    }
    let mut values = BTreeMap::new();
    let mut branches = BTreeMap::new();
    for id in plan {
        let n = v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == *id)
            .unwrap();
        let kind = n["kind"].as_str().unwrap();
        let p = &n["params"];
        let ps = parents(v, id, &values, &branches);
        if kind != "market" && ps.is_empty() {
            continue;
        }
        if kind == "if_else" {
            branches.insert(id.clone(), condition(p, features)?);
        }
        let result = if kind == "pine" {
            if incoming(&ps) == 0 {
                0
            } else {
                *external
                    .get(id)
                    .ok_or_else(|| anyhow::anyhow!("Pine-uitvoer ontbreekt"))?
            }
        } else {
            node_value(kind, p, &ps, closes, features, ts, last_order)?
        };
        values.insert(id.clone(), result);
    }
    let gate = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["kind"] == "risk_gate")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    Ok(match values.get(gate) {
        Some(1) => "BUY",
        Some(-1) => "SELL",
        _ => "HOLD",
    }
    .into())
}
pub async fn signal(a: &App, b: &Bot, data: &Value, revision: u64) -> Result<String> {
    let v:Value=sqlx::query_scalar("SELECT d.payload FROM studio.deployments p JOIN studio.documents d ON d.kind='graph' AND d.name=p.name AND d.version=p.version WHERE p.bot_id=$1").bind(&b.id).fetch_one(&a.db).await?;
    let plan = graph_plan(&v)?;
    validate_parameters(&v)?;
    let cs: Vec<_> = data["candles"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["closed"] == true)
        .collect();
    let ts = cs
        .last()
        .ok_or_else(|| anyhow::anyhow!("Geen gesloten candles"))?["ts"]
        .as_i64()
        .unwrap();
    let features = data["indicators"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["ts"] == ts)
        .ok_or_else(|| anyhow::anyhow!("Indicatoren ontbreken"))?;
    let mut features = features.clone();
    features["candle_open"] = cs.last().unwrap()["open"].clone();
    features["candle_close"] = cs.last().unwrap()["close"].clone();
    if let Some(stop) = candle_stop(&v, &features)? {
        return Ok(stop);
    }
    let features = &features;
    let closes: Vec<D> = cs
        .iter()
        .map(|c| c["close"].as_str().unwrap_or("").parse())
        .collect::<std::result::Result<_, _>>()?;
    let raw: Value = sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1")
        .fetch_one(&a.db)
        .await?;
    let last_order = raw["orders"]
        .as_object()
        .unwrap()
        .values()
        .filter(|o| o["intent"]["bot_id"] == b.id && o["intent"]["side"] == "BUY")
        .filter_map(|o| o["created_ms"].as_i64())
        .max()
        .unwrap_or(0);
    let mut values = BTreeMap::new();
    let mut branches = BTreeMap::new();
    for id in plan {
        let n = v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == id)
            .unwrap();
        let kind = n["kind"].as_str().unwrap();
        let p = &n["params"];
        let ps = parents(&v, &id, &values, &branches);
        if kind != "market" && ps.is_empty() {
            continue;
        }
        let incoming = incoming(&ps);
        if kind == "if_else" {
            branches.insert(id.clone(), condition(p, features)?);
        }
        let result = match kind {
            "pine" if incoming == 0 => 0,
            "pine" => {
                let doc:Value=sqlx::query_scalar("SELECT payload FROM studio.documents WHERE kind='pine' AND name=$1 AND version=$2").bind(p["name"].as_str().unwrap()).bind(p["version"].as_i64().unwrap()).fetch_one(&a.db).await?;
                let result = crate::studio::run_pine(
                    a,
                    doc["source"].as_str().unwrap_or(""),
                    &b.instrument,
                    &b.bar,
                    &doc["inputs"],
                )
                .await?;
                ensure!(result["ok"] == true, "Pine-runtime afgewezen");
                pine_value(&result, p, ts)?
            }
            "news" => {
                let received:Option<i64>=sqlx::query_scalar("SELECT received_ms FROM studio.providers WHERE name='okx_announcements' AND error IS NULL").fetch_optional(&a.db).await?;
                ensure!(
                    received.is_some_and(|t| now_ms() - t <= 900000),
                    "Nieuwsfilter: bron ontbreekt of verouderd"
                );
                let count: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM studio.news WHERE first_seen_ms>$1")
                        .bind(now_ms() - int(p, "quiet_minutes", 30, 5, 1440)? * 60000)
                        .fetch_one(&a.db)
                        .await?;
                if incoming < 0 || count == 0 {
                    incoming
                } else {
                    0
                }
            }
            "jev" => {
                if incoming <= 0 {
                    incoming
                } else {
                    // Deterministic prefilter before paid AI call; fixed limits are checked again by submit.
                    let qs = paper::quotes(a).await;
                    let q = qs
                        .get(&b.instrument)
                        .ok_or_else(|| anyhow::anyhow!("Jev-prefilter: geen verse quote"))?;
                    let raw: Value =
                        sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1")
                            .fetch_one(&a.db)
                            .await?;
                    let paper: Paper = serde_json::from_value(raw)?;
                    ensure!(
                        paper.revision == revision && !paper.ledger.paused,
                        "Rekening gewijzigd"
                    );
                    paper.preview(&strategy_intent(b, q, ts), q, now_ms())?;
                    ensure!(
                        b.limits.ai_budget > D::ZERO && paper.limits.ai_budget > D::ZERO,
                        "AI-budget van bot/rekening is nul"
                    );
                    let d = crate::providers::decide(
                        a,
                        &b.instrument,
                        &b.bar,
                        &b.version,
                        Some(revision),
                    )
                    .await?;
                    ensure!(
                        d["usable"] == true
                            && d["expires_at_ms"].as_i64().is_some_and(|t| now_ms() < t),
                        "Jev-antwoord verlopen of onbruikbaar"
                    );
                    match d["answer"].as_str() {
                        Some("CONSIDER_ENTRY") => 1,
                        Some("REDUCE") => -1,
                        _ => 0,
                    }
                }
            }

            _ => node_value(kind, p, &ps, &closes, features, ts, last_order)?,
        };
        values.insert(id, result);
    }
    let gate = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["kind"] == "risk_gate")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    Ok(match values.get(gate) {
        Some(1) => "BUY",
        Some(-1) => "SELL",
        _ => "HOLD",
    }
    .into())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Install {
    name: String,
    version: i64,
}
async fn install(State(a): State<App>, Json(r): Json<Install>) -> Response {
    reply(async {
    let v:Value=sqlx::query_scalar("SELECT payload FROM studio.documents WHERE kind='graph' AND name=$1 AND version=$2").bind(&r.name).bind(r.version).fetch_one(&a.db).await?;
    graph_plan(&v)?;validate_parameters(&v)?;
    ensure!(v["bot"]["mode"]=="paper","Live-uitvoering is niet vrijgegeven; liveconfiguratie blijft een opgeslagen ontwerp");
    ensure!(["1m", "5m", "15m"].contains(&v["bot"]["bar"].as_str().unwrap_or("1m")),"Botinterval ongeldig");
    let mut config:paper::BotConfig=serde_json::from_value(v["bot"]["config"].clone())?;config.bar=v["bot"]["bar"].as_str().unwrap_or("1m").into();
    let id=config.id.clone();ensure!(id!="manual","Kies een eigen bot-id");
    ensure!(paper::quotes(&a).await.contains_key(&config.instrument),"Kies een beschikbare EUR-spotmarkt");
    for n in v["nodes"].as_array().unwrap().iter().filter(|n|n["kind"]=="pine") {
        let p=&n["params"];
        let doc:Value=sqlx::query_scalar("SELECT payload FROM studio.documents WHERE kind='pine' AND name=$1 AND version=$2").bind(p["name"].as_str().unwrap()).bind(p["version"].as_i64().unwrap()).fetch_one(&a.db).await?;
        let result=crate::studio::run_pine(&a,doc["source"].as_str().unwrap_or(""),&config.instrument,&config.bar,&doc["inputs"]).await?;
        ensure!(result["ok"]==true,"Pine-fout: {}",result["diagnostics"]);
        ensure!(p["plot"].as_str().is_some_and(|t|!t.is_empty()) || result["plots"].as_array().is_some_and(|ps|ps.iter().any(|x|x["title"]=="entry"||x["title"]=="exit")),"Kies een indicatorplot of voeg entry/exit-signalen aan het script toe");
        if let Some(ts)=result["plots"].as_array().and_then(|ps|ps.first()).and_then(|p|p["data"].as_array()).and_then(|ds|ds.last()).and_then(|d|d["time"].as_i64()) {pine_value(&result,p,ts)?;}
    }
    let mut tx=a.db.begin().await?;let mut p=paper::load(&mut tx).await?;let now=now_ms();
    let previous=p.bots.get(&id).cloned();
    if let Some(old)=&previous {
        ensure!(!old.running,"Pauzeer de bot voordat je instellingen toepast");
        ensure!(!p.positions.values().any(|x|x.bot_id==id&&x.quantity>D::ZERO)&&!p.orders.values().any(|x|x.intent.bot_id==id&&x.state.open()),"Sluit eerst posities en open orders van deze bot");
        ensure!(old.limits.capital==config.limits.capital,"Bestaand botkapitaal blijft vast; maak een kopie voor een ander budget");
        p.bots.remove(&id);
    }
    p.add_bot(config.bot(now)?,now)?;
    let b=p.bots.get_mut(&id).unwrap();
    if let Some(old)=previous {b.ledger=old.ledger;}
    b.version=format!("graph:{}:{}",r.name,r.version);b.last_signal="Opgeslagen instellingen toegepast; gepauzeerd".into();b.last_bar=(now/60000-1)*60000;
    sqlx::query("INSERT INTO studio.deployments(bot_id,name,version,created_ms) VALUES($1,$2,$3,$4) ON CONFLICT(bot_id) DO UPDATE SET name=$2,version=$3,created_ms=$4").bind(&id).bind(&r.name).bind(r.version).bind(now).execute(&mut *tx).await?;
    p.event(now,"DEPLOY_GRAPH",&id,&format!("{} v{}",r.name,r.version));paper::persist(&mut tx,&mut p).await?;tx.commit().await?;
    Ok(json!({"ok":true,"bot_id":id,"running":false}))
 }.await)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn branch() -> Value {
        json!({"schema":1,"nodes":[{"id":"m","kind":"market"},{"id":"f","kind":"if_else","params":{"field":"rsi","comparison":"above","value":50}},{"id":"yes","kind":"action","params":{"action":"buy"}},{"id":"no","kind":"action","params":{"action":"sell"}},{"id":"join","kind":"or"},{"id":"i","kind":"intent"},{"id":"r","kind":"risk_gate"}],"edges":[{"source":"m","target":"f"},{"source":"f","target":"yes","sourceHandle":"true"},{"source":"f","target":"no","sourceHandle":"false"},{"source":"yes","target":"join"},{"source":"no","target":"join"},{"source":"join","target":"i"},{"source":"i","target":"r"}]})
    }
    fn run(v: &Value, rsi: f64) -> String {
        evaluate(
            v,
            &graph_plan(v).unwrap(),
            &[],
            &json!({"rsi":rsi}),
            600000,
            0,
            &BTreeMap::new(),
        )
        .unwrap()
    }
    #[test]
    fn branches_are_exclusive_and_roundtrip_preserves_handles() {
        let v = branch();
        assert_eq!(run(&v, 60.), "BUY");
        assert_eq!(run(&v, 40.), "SELL");
        let loaded: Value = serde_json::from_str(&v.to_string()).unwrap();
        assert_eq!(run(&loaded, 40.), "SELL");
    }
    #[test]
    fn missing_or_unknown_branch_handles_rejected() {
        let mut v = branch();
        v["edges"][1]["sourceHandle"] = json!("wrong");
        assert!(graph_plan(&v).is_err());
        v["edges"][1]["sourceHandle"] = Value::Null;
        assert!(graph_plan(&v).is_err());
    }
    #[test]
    fn both_if_else_paths_must_be_connected() {
        let mut v = branch();
        v["edges"][2]["sourceHandle"] = json!("true");
        assert!(graph_plan(&v).is_err());
    }
    #[test]
    fn not_cannot_activate_unchosen_branch() {
        let mut v = branch();
        v["nodes"][3]["kind"] = json!("not");
        assert_eq!(run(&v, 60.), "BUY");
        assert_eq!(run(&v, 40.), "HOLD");
    }
    #[test]
    fn comparison_boundary_is_false() {
        assert_eq!(run(&branch(), 50.), "SELL");
    }
    #[test]
    fn warmup_cannot_invent_branch() {
        let v = branch();
        assert!(
            evaluate(
                &v,
                &graph_plan(&v).unwrap(),
                &[],
                &json!({"rsi":null}),
                600000,
                0,
                &BTreeMap::new()
            )
            .is_err()
        );
    }
    #[test]
    fn unknown_action_rejected() {
        let mut v = branch();
        v["nodes"][2]["params"]["action"] = json!("leverage");
        assert!(validate_parameters(&v).is_err());
    }
    #[test]
    fn exit_wins_for_pine_and_plot_comparison_is_explicit() {
        let r = json!({"plots":[{"title":"RSI","data":[{"time":1,"value":65}]}],"intents":[{"bar_ms":1,"action":"CONSIDER_ENTRY"},{"bar_ms":1,"action":"CONSIDER_EXIT"}]});
        assert_eq!(pine_value(&r, &json!({}), 1).unwrap(), -1);
        assert_eq!(
            pine_value(
                &r,
                &json!({"plot":"RSI","comparison":"above","value":60}),
                1
            )
            .unwrap(),
            1
        );
        assert_eq!(
            pine_value(
                &r,
                &json!({"plot":"RSI","comparison":"above","value":70}),
                1
            )
            .unwrap(),
            0
        );
        assert!(pine_value(&r, &json!({"plot":"Missing"}), 1).is_err());
    }
    #[test]
    fn cooldown_uses_order_time_and_does_not_block_exit() {
        let p = json!({"bars":5});
        assert_eq!(
            node_value("cooldown", &p, &[1], &[], &Value::Null, 600000, 550000).unwrap(),
            0
        );
        assert_eq!(
            node_value("cooldown", &p, &[-1], &[], &Value::Null, 600000, 550000).unwrap(),
            -1
        );
        assert_eq!(
            node_value("cooldown", &p, &[1], &[], &Value::Null, 900000, 550000).unwrap(),
            1
        );
    }
    #[test]
    fn system_catalog_is_valid_and_has_no_external_filters() {
        let catalog: Value = serde_json::from_str(include_str!("../system_bots.json")).unwrap();
        assert_eq!(catalog.as_array().unwrap().len(), 5);
        for item in catalog.as_array().unwrap() {
            let g = &item["graph"];
            graph_plan(g).unwrap();
            validate_parameters(g).unwrap();
            assert!(
                !g["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|n| ["jev", "news", "pine"].contains(&n["kind"].as_str().unwrap()))
            );
        }
    }
    #[test]
    fn explicit_ai_permission_is_required() {
        let mut g = branch();
        g["nodes"][2]["kind"] = json!("jev");
        assert!(
            validate_parameters(&g)
                .unwrap_err()
                .to_string()
                .contains("AI staat uit")
        );
    }
    #[test]
    fn standard_crosses_and_warmup() {
        for kind in ["ema_cross", "macd_cross"] {
            let mut xs = vec![D::from(100); 40];
            assert_eq!(standard_signal(kind, &json!({}), &xs).unwrap(), 0);
            xs.push(D::from(101));
            assert_eq!(standard_signal(kind, &json!({}), &xs).unwrap(), 1);
            *xs.last_mut().unwrap() = D::from(99);
            assert_eq!(standard_signal(kind, &json!({}), &xs).unwrap(), -1);
            assert_eq!(standard_signal(kind, &json!({}), &xs[..2]).unwrap(), 0);
        }
        let mut xs = vec![D::from(100); 19];
        xs.push(D::from(90));
        assert_eq!(standard_signal("bollinger", &json!({}), &xs).unwrap(), 1);
        *xs.last_mut().unwrap() = D::from(101);
        assert_eq!(standard_signal("bollinger", &json!({}), &xs).unwrap(), -1);
    }
    #[test]
    fn candle_stop_threshold_and_priority() {
        let catalog: Value = serde_json::from_str(include_str!("../system_bots.json")).unwrap();
        let mut g = catalog[0]["graph"].clone();
        assert_eq!(
            candle_stop(&g, &json!({"candle_open":"100","candle_close":"99"})).unwrap(),
            Some("STOP_CLOSE".into())
        );
        assert_eq!(
            candle_stop(&g, &json!({"candle_open":"100","candle_close":"99.01"})).unwrap(),
            None
        );
        assert_eq!(
            candle_stop(
                &g,
                &json!({"candle_open":"100","candle_close":"100","low":"90"})
            )
            .unwrap(),
            None
        );
        // A stop fires without indicator warm-up or an incoming BUY.
        assert_eq!(
            evaluate(
                &g,
                &graph_plan(&g).unwrap(),
                &[],
                &json!({"candle_open":"100","candle_close":"99"}),
                60000,
                0,
                &BTreeMap::new()
            )
            .unwrap(),
            "STOP_CLOSE"
        );
        g["nodes"][2]["params"]["action"] = json!("pause");
        assert_eq!(
            candle_stop(&g, &json!({"candle_open":"100","candle_close":"98"})).unwrap(),
            Some("STOP_PAUSE".into())
        );
        g["nodes"][2]["params"]["action"] = json!("close");
        assert_eq!(
            candle_stop(&g, &json!({"candle_open":"100","candle_close":"98"})).unwrap(),
            Some("EXIT_NOW".into())
        );
        assert!(candle_stop(&g, &json!({})).is_err());
        g["nodes"][2]["params"]["drop_pct"] = json!(-1);
        assert!(validate_parameters(&g).is_err());
    }
}
