//! Immutable graph deployments and bounded proposals for the existing fixed risk core.
use crate::{
    App,
    model::now_ms,
    paper,
    risk::{Bot, Paper, sma_signal, strategy_intent},
    studio::{graph_plan, reply},
};
use anyhow::{Result, bail, ensure};
use axum::{Json, Router, extract::State, response::Response, routing::post};
use rust_decimal::Decimal as D;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub fn router() -> Router<App> {
    Router::new().route("/api/studio/deploy", post(deploy))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Deploy {
    bot_id: String,
    name: String,
    version: i64,
}
async fn deploy(State(a): State<App>, Json(r): Json<Deploy>) -> Response {
    reply(async{ensure!(r.bot_id!="manual","Handmatig budget is geen strategie-deployment");let v:Value=sqlx::query_scalar("SELECT payload FROM studio.documents WHERE kind='graph' AND name=$1 AND version=$2").bind(&r.name).bind(r.version).fetch_one(&a.db).await?;graph_plan(&v)?;validate_parameters(&v)?;let mut tx=a.db.begin().await?;let mut p=paper::load(&mut tx).await?;ensure!(!p.positions.values().any(|x|x.bot_id==r.bot_id&&x.quantity>D::ZERO)&&!p.orders.values().any(|x|x.intent.bot_id==r.bot_id&&x.state.open()),"Sluit exposure en orders voordat een andere versie wordt gekoppeld");let b=p.bots.get_mut(&r.bot_id).ok_or_else(||anyhow::anyhow!("Bot bestaat niet"))?;ensure!(!b.running,"Pauzeer de bot vóór een versie-wijziging");b.version=format!("graph:{}:{}",r.name,r.version);b.last_signal="Nieuwe versie gekoppeld; wacht op expliciete start".into();b.last_bar=(now_ms()/60000-1)*60000;sqlx::query("INSERT INTO studio.deployments(bot_id,name,version,created_ms) VALUES($1,$2,$3,$4) ON CONFLICT(bot_id) DO UPDATE SET name=$2,version=$3,created_ms=$4").bind(&r.bot_id).bind(&r.name).bind(r.version).bind(now_ms()).execute(&mut *tx).await?;p.event(now_ms(),"DEPLOY_GRAPH",&r.bot_id,&format!("{} v{}",r.name,r.version));paper::persist(&mut tx,&mut p).await?;tx.commit().await?;Ok(json!({"ok":true}))}.await)
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
    for n in v["nodes"].as_array().unwrap() {
        let p = &n["params"];
        match n["kind"].as_str().unwrap_or("") {
            "sma_cross" => {
                ensure!(
                    int(p, "fast", 9, 2, 99)? < int(p, "slow", 21, 3, 100)?,
                    "SMA fast moet kleiner dan slow zijn"
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
            "condition" => {
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
                    p["value"].as_f64().unwrap_or(50.).is_finite(),
                    "Drempel ongeldig"
                );
            }
            "pine" => {
                ensure!(
                    p["name"].as_str().is_some() && p["version"].as_i64().is_some_and(|v| v > 0),
                    "Pine-node vereist opgeslagen naam en exacte versie"
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
    let last = cs
        .last()
        .ok_or_else(|| anyhow::anyhow!("Geen gesloten candle"))?;
    let ts = last["ts"].as_i64().unwrap();
    let features = data["indicators"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["ts"] == ts)
        .ok_or_else(|| anyhow::anyhow!("Indicatoren ontbreken"))?;
    let mut values = BTreeMap::<String, i8>::new();
    for id in plan {
        let n = v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == id)
            .unwrap();
        let p = &n["params"];
        let parents: Vec<i8> = v["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["target"] == id)
            .filter_map(|e| values.get(e["source"].as_str().unwrap()).copied())
            .collect();
        let incoming = if parents.iter().any(|p| *p < 0) {
            -1
        } else if parents.iter().all(|p| *p > 0) {
            1
        } else {
            0
        };
        let result = if incoming == 0
            && ["sma_cross", "rsi_range", "pine"].contains(&n["kind"].as_str().unwrap_or(""))
        {
            0
        } else {
            match n["kind"].as_str().unwrap() {
                "market" => 1,
                "sma_cross" => {
                    let closes: Vec<D> = cs
                        .iter()
                        .map(|c| c["close"].as_str().unwrap_or("").parse())
                        .collect::<std::result::Result<_, _>>()?;
                    match sma_signal(
                        &closes,
                        int(p, "fast", 9, 2, 99)? as usize,
                        int(p, "slow", 21, 3, 100)? as usize,
                    ) {
                        Some("BUY") => 1,
                        Some("SELL") => -1,
                        _ => 0,
                    }
                }
                "rsi_range" => {
                    let r = features["rsi"]
                        .as_f64()
                        .ok_or_else(|| anyhow::anyhow!("RSI warm-up ontbreekt"))?;
                    if r < f64::from(int(p, "low", 30, 1, 49)? as i32) {
                        1
                    } else if r > f64::from(int(p, "high", 70, 51, 99)? as i32) {
                        -1
                    } else {
                        0
                    }
                }
                "condition" => {
                    let f = p["field"].as_str().unwrap_or("rsi");
                    let val = features[f]
                        .as_f64()
                        .ok_or_else(|| anyhow::anyhow!("Indicator warm-up ontbreekt"))?;
                    let threshold = p["value"].as_f64().unwrap_or(50.);
                    let cond = if p["comparison"] == "below" {
                        val < threshold
                    } else {
                        val > threshold
                    };
                    if incoming < 0 || cond { incoming } else { 0 }
                }
                "and" => incoming,
                "or" => {
                    if parents.contains(&-1) {
                        -1
                    } else if parents.contains(&1) {
                        1
                    } else {
                        0
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
                    if incoming < 0 || active { incoming } else { 0 }
                }
                "cooldown" => {
                    let raw: Value =
                        sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1")
                            .fetch_one(&a.db)
                            .await?;
                    let orders = raw["orders"]
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("Orderstaat ontbreekt"))?;
                    let last_order = orders
                        .values()
                        .filter(|o| o["intent"]["bot_id"] == b.id && o["intent"]["side"] == "BUY")
                        .filter_map(|o| o["created_ms"].as_i64())
                        .max()
                        .unwrap_or(0);
                    if incoming < 0 || now_ms() - last_order >= int(p, "bars", 5, 1, 1440)? * 60000
                    {
                        incoming
                    } else {
                        0
                    }
                }
                "news" => {
                    let received:Option<i64>=sqlx::query_scalar("SELECT received_ms FROM studio.providers WHERE name='okx_announcements' AND error IS NULL").fetch_optional(&a.db).await?;
                    ensure!(
                        received.is_some_and(|t| now_ms() - t <= 900000),
                        "Nieuwsfilter: bron ontbreekt of verouderd"
                    );
                    let count: i64 = sqlx::query_scalar(
                        "SELECT count(*) FROM studio.news WHERE first_seen_ms>$1",
                    )
                    .bind(now_ms() - int(p, "quiet_minutes", 30, 5, 1440)? * 60000)
                    .fetch_one(&a.db)
                    .await?;
                    if incoming < 0 || count == 0 {
                        incoming
                    } else {
                        0
                    }
                }
                "pine" => {
                    let doc:Value=sqlx::query_scalar("SELECT payload FROM studio.documents WHERE kind='pine' AND name=$1 AND version=$2").bind(p["name"].as_str().unwrap()).bind(p["version"].as_i64().unwrap()).fetch_one(&a.db).await?;
                    let result = crate::studio::run_pine(
                        a,
                        doc["source"].as_str().unwrap_or(""),
                        &b.instrument,
                        "1m",
                        &doc["inputs"],
                    )
                    .await?;
                    ensure!(result["ok"] == true, "Pine-runtime afgewezen");
                    let mut direction = 0;
                    for i in result["intents"].as_array().unwrap() {
                        if i["bar_ms"] == ts {
                            if i["action"] == "CONSIDER_EXIT" {
                                direction = -1
                            } else if direction != -1 {
                                direction = 1
                            }
                        }
                    }
                    direction
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
                            "1m",
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
                "sizing" | "intent" | "risk_gate" => incoming,
                _ => bail!("Onbekende node"),
            }
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
    Ok(match values[gate] {
        1 => "BUY",
        -1 => "SELL",
        _ => "HOLD",
    }
    .into())
}
