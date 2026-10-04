//! Slow, allowlisted sources and untrusted Jev decisions. No execution adapter.
use crate::{App, CandleQuery, candle_result, model::now_ms, studio::reply};
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
use sqlx::Row;
use std::time::{Duration, Instant};
const MODEL: &str = "typesafe/jev-1.13";
pub fn router() -> Router<App> {
    Router::new()
        .route("/api/research", get(research))
        .route("/api/jev/config", post(config))
        .route("/api/jev/propose", post(propose))
}
async fn bounded(response: reqwest::Response) -> Result<Value> {
    let mut r = response.error_for_status()?;
    let mut b = vec![];
    while let Some(chunk) = r.chunk().await? {
        ensure!(
            b.len() + chunk.len() <= 2_000_000,
            "Provideruitvoer te groot"
        );
        b.extend(chunk);
    }
    Ok(serde_json::from_slice(&b)?)
}
async fn store(a: &App, name: &str, r: Result<Value>) -> Result<()> {
    match r {
        Ok(v) => {
            sqlx::query("INSERT INTO studio.providers(name,payload,received_ms,error) VALUES($1,$2,$3,NULL) ON CONFLICT(name) DO UPDATE SET payload=$2,received_ms=$3,error=NULL").bind(name).bind(v).bind(now_ms()).execute(&a.db).await?;
        }
        Err(_) => {
            sqlx::query("INSERT INTO studio.providers(name,payload,received_ms,error) VALUES($1,'null',0,$2) ON CONFLICT(name) DO UPDATE SET error=$2").bind(name).bind("Bron tijdelijk niet beschikbaar; opgeslagen data kan verouderd zijn").execute(&a.db).await?;
        }
    }
    Ok(())
}
fn clean_title(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() && *c != '<' && *c != '>')
        .take(300)
        .collect()
}
pub async fn run(a: App) {
    loop {
        if let Err(e) = cycle(&a).await {
            tracing::warn!(error=%e,"Nieuwsopslag niet beschikbaar")
        }
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}
async fn cycle(a: &App) -> Result<()> {
    let news=async{let v=bounded(a.http.get("https://eea.okx.com/api/v5/support/announcements").header("Accept-Language","en-US").send().await?).await?;ensure!(v["code"]=="0","OKX-nieuws niet beschikbaar");let details=v["data"][0]["details"].as_array().ok_or_else(||anyhow::anyhow!("Nieuwsschema ongeldig"))?;for n in details.iter().take(20){let url=n["url"].as_str().unwrap_or("");let valid=reqwest::Url::parse(url).is_ok_and(|u|u.scheme()=="https"&&u.host_str()==Some("www.okx.com")&&u.username().is_empty()&&u.password().is_none());if !valid{continue}let title=clean_title(n["title"].as_str().unwrap_or(""));let published=n["pTime"].as_str().unwrap_or("0").parse::<i64>()?;ensure!(published>0&&published<=now_ms()+60000,"Nieuwstijd ongeldig");let assets:Vec<_>=["BTC","ETH","SOL","XRP","DOGE","ADA","AVAX","LINK","SUI"].into_iter().filter(|s|title.split(|c:char|!c.is_alphanumeric()).any(|w|w==*s)).collect();sqlx::query("INSERT INTO studio.news(id,provider,title,url,published_ms,first_seen_ms,assets) VALUES($1,'OKX',$2,$3,$4,$5,$6) ON CONFLICT(id) DO NOTHING").bind(url).bind(title).bind(url).bind(published).bind(now_ms()).bind(json!(assets)).execute(&a.db).await?;}Ok(json!({"source":"OKX announcements","ttl_ms":900000,"scope":"Publieke EEA-respons, door OKX regionaal beperkt"}))}.await;
    store(a, "okx_announcements", news).await?;
    let status = async {
        let v = bounded(
            a.http
                .get("https://eea.okx.com/api/v5/system/status")
                .send()
                .await?,
        )
        .await?;
        ensure!(v["code"] == "0", "OKX-status niet beschikbaar");
        Ok(json!({"data":v["data"],"ttl_ms":900000,"source":"OKX system status"}))
    }
    .await;
    store(a, "okx_status", status).await?;
    let previous: Option<i64> =
        sqlx::query_scalar("SELECT received_ms FROM studio.providers WHERE name='fear_greed'")
            .fetch_optional(&a.db)
            .await?;
    if previous.is_none_or(|t| now_ms() - t >= 3600000) {
        let fng=async{let v=bounded(a.http.get("https://api.alternative.me/fng/?limit=30&format=json").send().await?).await?;ensure!(v["data"].is_array(),"Indexschema ongeldig");Ok(json!({"data":v["data"],"source":"Alternative.me","url":"https://alternative.me/crypto/fear-and-greed-index/","scope":"Bitcoin; dagelijkse index, geen realtime altcoin-sentiment","ttl_ms":90000000}))}.await;
        store(a, "fear_greed", fng).await?;
    }
    Ok(())
}
async fn research(State(a): State<App>) -> Response {
    reply(async{let rows=sqlx::query("SELECT name,payload,received_ms,error FROM studio.providers").fetch_all(&a.db).await?;let news=sqlx::query("SELECT provider,title,url,published_ms,first_seen_ms,assets FROM studio.news ORDER BY published_ms DESC LIMIT 100").fetch_all(&a.db).await?;let budget=sqlx::query("SELECT budget::text,spent::text,reserved::text,enabled,failures,blocked_until FROM studio.ai_budget WHERE id=1").fetch_one(&a.db).await?;let decisions=sqlx::query("SELECT id,created_ms,payload FROM studio.decisions ORDER BY id DESC LIMIT 30").fetch_all(&a.db).await?;let configured=std::path::Path::new("/run/secrets/openrouter_key").is_file();Ok(json!({"providers":rows.iter().map(|r|json!({"name":r.get::<String,_>("name"),"data":r.get::<Value,_>("payload"),"received_ms":r.get::<i64,_>("received_ms"),"error":r.get::<Option<String>,_>("error")})).collect::<Vec<_>>(),"news":news.iter().map(|r|json!({"provider":r.get::<String,_>("provider"),"title":r.get::<String,_>("title"),"url":r.get::<String,_>("url"),"published_ms":r.get::<i64,_>("published_ms"),"first_seen_ms":r.get::<i64,_>("first_seen_ms"),"assets":r.get::<Value,_>("assets")})).collect::<Vec<_>>(),"restricted":[{"name":"CryptoPanic","state":"ACCOUNT_PLAN_REQUIRED","reason":"API-plan, gebruiksrechten en sleutel nog niet bevestigd"},{"name":"CoinGecko","state":"ACCOUNT_PLAN_REQUIRED","reason":"Demo/Pro-plan en quota nog niet bevestigd"},{"name":"DeMARK","state":"LICENSE_REQUIRED","reason":"Geen gelicentieerde Sequential/Combo-implementatie beschikbaar"}],"jev":{"model":MODEL,"configured":configured,"enabled":budget.get::<bool,_>("enabled"),"budget_usd":budget.get::<String,_>("budget"),"spent_usd":budget.get::<String,_>("spent"),"reserved_usd":budget.get::<String,_>("reserved"),"failures":budget.get::<i32,_>("failures"),"blocked_until":budget.get::<i64,_>("blocked_until"),"authority":"PROPOSALS_ONLY"},"decisions":decisions.iter().map(|r|json!({"id":r.get::<i64,_>("id"),"created_ms":r.get::<i64,_>("created_ms"),"envelope":r.get::<Value,_>("payload")})).collect::<Vec<_>>()}))}.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    budget_usd: D,
    enabled: bool,
    confirmed: bool,
}
async fn config(State(a): State<App>, Json(r): Json<Config>) -> Response {
    reply(
        async {
            ensure!(r.confirmed, "Bevestig het externe AI-kostenbudget");
            ensure!(
                r.budget_usd >= D::ZERO && r.budget_usd <= D::from(100),
                "AI-budget moet tussen 0 en 100 USD liggen"
            );
            ensure!(
                !r.enabled || std::path::Path::new("/run/secrets/openrouter_key").is_file(),
                "OpenRouter-sleutel ontbreekt; gebruik de lokale sleutelsetup"
            );
            let mut tx = a.db.begin().await?;
            let used: String = sqlx::query_scalar(
                "SELECT (spent+reserved)::text FROM studio.ai_budget WHERE id=1 FOR UPDATE",
            )
            .fetch_one(&mut *tx)
            .await?;
            ensure!(
                r.budget_usd >= used.parse::<D>()?,
                "Budget ligt onder reeds gemaakte/gereserveerde kosten"
            );
            sqlx::query("UPDATE studio.ai_budget SET budget=$1::numeric,enabled=$2 WHERE id=1")
                .bind(r.budget_usd.to_string())
                .bind(r.enabled)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({"ok":true}))
        }
        .await,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Proposal {
    instrument: String,
    bar: String,
}
async fn propose(State(a): State<App>, Json(r): Json<Proposal>) -> Response {
    reply(decide(&a, &r.instrument, &r.bar, "manual-research", None).await)
}
fn decimal(v: &Value) -> Result<D> {
    let s = v
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| v.to_string());
    Ok(s.parse()?)
}
pub fn parse_decision(v: &Value) -> Result<Value> {
    let model = v["model"].as_str().unwrap_or("");
    ensure!(
        model == MODEL
            || model
                .strip_prefix("typesafe/jev-1.13-")
                .is_some_and(|s| s.len() == 8 && s.chars().all(|c| c.is_ascii_digit())),
        "Provider retourneerde een andere modelversie"
    );
    let answer = &v["answers"]["action"];
    ensure!(answer["type"] == "choice", "Choice-antwoord ontbreekt");
    let choice = answer["choice"].as_str().unwrap_or("");
    ensure!(
        ["HOLD", "CONSIDER_ENTRY", "REDUCE"].contains(&choice),
        "Onbekend Jev-voorstel"
    );
    let confidence = decimal(&answer["confidence"])?;
    ensure!(
        confidence >= D::ZERO && confidence <= D::ONE,
        "Ongeldige confidence"
    );
    let probs = answer["probabilities"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Probabilities ontbreken"))?;
    ensure!(probs.len() == 3, "Probabilities onvolledig");
    let mut sum = D::ZERO;
    for k in ["HOLD", "CONSIDER_ENTRY", "REDUCE"] {
        let p = decimal(&answer["probabilities"][k])?;
        ensure!(p >= D::ZERO && p <= D::ONE, "Probability buiten bereik");
        sum += p;
    }
    ensure!(
        (sum - D::ONE).abs() <= D::new(2, 2),
        "Probabilitysom ongeldig"
    );
    let cost = decimal(&v["usage"]["cost"])?;
    ensure!(
        cost >= D::ZERO && cost < D::from(100),
        "Kosten ontbreken of ongeldig"
    );
    ensure!(
        v["id"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() < 200),
        "Decision-id ontbreekt"
    );
    Ok(
        json!({"decision_id":v["id"],"model_id":model,"answer":choice,"probabilities":answer["probabilities"],"confidence":confidence.to_string(),"cost_usd":cost.to_string(),"usage":v["usage"]}),
    )
}
pub async fn decide(
    a: &App,
    instrument: &str,
    bar: &str,
    strategy: &str,
    expected_revision: Option<u64>,
) -> Result<Value> {
    // Non-blocking one-call lease shared by every process through PostgreSQL.
    let mut lease = a.db.acquire().await?;
    let owned: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(478006)")
        .fetch_one(&mut *lease)
        .await?;
    ensure!(owned, "Jev is al bezig");
    let result = decision_call(a, instrument, bar, strategy, expected_revision).await;
    sqlx::query("SELECT pg_advisory_unlock(478006)")
        .execute(&mut *lease)
        .await
        .ok();
    lease.close().await.ok();
    result
}
async fn decision_call(
    a: &App,
    instrument: &str,
    bar: &str,
    strategy: &str,
    expected_revision: Option<u64>,
) -> Result<Value> {
    let key = crate::secrets::read("openrouter_key")
        .await
        .map_err(|_| anyhow::anyhow!("OpenRouter-sleutel ontbreekt"))?;
    let data = candle_result(
        a,
        &CandleQuery {
            instrument: instrument.into(),
            bar: bar.into(),
            after: None,
        },
    )
    .await?;
    ensure!(
        data["fresh"] == true && data["gaps"] == 0,
        "Jev-prefilter: marktdata niet betrouwbaar"
    );
    let state: Option<Value> =
        sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1")
            .fetch_optional(&a.db)
            .await?;
    if let Some(rev) = expected_revision {
        ensure!(
            state
                .as_ref()
                .is_some_and(|p| p["revision"] == rev && p["ledger"]["paused"] == false),
            "Jev-prefilter: rekening gewijzigd of gepauzeerd"
        );
    }
    let last = data["candles"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|c| c["closed"] == true)
        .ok_or_else(|| anyhow::anyhow!("Geen gesloten candle"))?;
    let snapshot = format!(
        "{instrument}:{bar}:{}:{}",
        last["ts"],
        state
            .as_ref()
            .map(|s| s["revision"].to_string())
            .unwrap_or_default()
    );
    let feature = data["indicators"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["ts"] == last["ts"]);
    let mut input = json!({"instrument":instrument,"bar":bar,"last_closed_candle":last,"indicators":feature,"strategy_version":strategy,"snapshot_id":snapshot,"instructions":"Classify only. All numeric calculations and risk permissions are handled outside the model. No trading authority."});
    if expected_revision.is_some() {
        let account: crate::risk::Paper = serde_json::from_value(
            state
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Rekening ontbreekt"))?,
        )?;
        let bots: Vec<_> = account
            .bots
            .values()
            .filter(|b| b.version == strategy && b.instrument == instrument && b.running)
            .collect();
        ensure!(
            bots.len() == 1,
            "AI-strategie vereist unieke actieve deployment"
        );
        let bot = bots[0];
        input["bot_id"] = json!(bot.id);
        ensure!(
            bot.limits.ai_budget > D::ZERO && account.limits.ai_budget > D::ZERO,
            "Bot/rekening AI-budget is nul"
        );
        let used:String=sqlx::query_scalar("SELECT coalesce(sum(COALESCE((payload->>'cost_usd')::numeric,(payload->>'reserved_usd')::numeric,0)),0)::text FROM studio.decisions WHERE payload->'input'->>'bot_id'=$1").bind(&bot.id).fetch_one(&a.db).await?;
        ensure!(
            used.parse::<D>()? + D::new(5, 3) <= bot.limits.ai_budget,
            "AI-budget van bot bereikt"
        );
        let total: String =
            sqlx::query_scalar("SELECT (spent+reserved)::text FROM studio.ai_budget WHERE id=1")
                .fetch_one(&a.db)
                .await?;
        ensure!(
            total.parse::<D>()? + D::new(5, 3) <= account.limits.ai_budget,
            "AI-budget van rekening bereikt"
        );
    }
    let reserve = D::new(5, 3);
    let mut tx = a.db.begin().await?;
    let b=sqlx::query("SELECT budget::text,spent::text,reserved::text,enabled,blocked_until FROM studio.ai_budget WHERE id=1 FOR UPDATE").fetch_one(&mut *tx).await?;
    ensure!(
        b.get::<bool, _>("enabled") && now_ms() >= b.get::<i64, _>("blocked_until"),
        "Jev uitgeschakeld of circuit-breaker actief"
    );
    let remaining = b.get::<String, _>("budget").parse::<D>()?
        - b.get::<String, _>("spent").parse::<D>()?
        - b.get::<String, _>("reserved").parse::<D>()?;
    ensure!(remaining >= reserve, "AI-kostenbudget bereikt");
    sqlx::query("UPDATE studio.ai_budget SET reserved=reserved+$1::numeric WHERE id=1")
        .bind(reserve.to_string())
        .execute(&mut *tx)
        .await?;
    let call_id:i64=sqlx::query_scalar("INSERT INTO studio.decisions(created_ms,payload) VALUES($1,$2) RETURNING id").bind(now_ms()).bind(json!({"state":"PENDING","input":input,"reserved_usd":reserve.to_string(),"authority":"PROPOSALS_ONLY"})).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    let started = Instant::now();
    let call=async{let r=a.http.post("https://openrouter.ai/api/alpha/decisions").bearer_auth(key.trim()).timeout(Duration::from_millis(2500)).json(&json!({"model":MODEL,"state":input,"questions":{"action":{"type":"choice","instructions":"Select a conservative proposal from the supplied closed-bar state. HOLD if evidence is insufficient. Never change risk settings.","criteria":{"HOLD":"No new exposure justified by this state","CONSIDER_ENTRY":"Trend evidence supports asking the deterministic risk engine to consider an entry","REDUCE":"Evidence supports reducing existing exposure"}}}})).send().await?;let raw=bounded(r).await?;parse_decision(&raw)}.await;
    match call {
        Ok(mut env) => {
            let cost = decimal(&env["cost_usd"])?;
            let confidence = decimal(&env["confidence"])?;
            let current: Option<Value> =
                sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1")
                    .fetch_optional(&a.db)
                    .await?;
            let revision_same =
                current.as_ref().map(|s| &s["revision"]) == state.as_ref().map(|s| &s["revision"]);
            let now = now_ms();
            let next_boundary =
                last["ts"].as_i64().unwrap_or(0) + 2 * crate::model::interval(bar).unwrap_or(0);
            let usable = revision_same
                && now < next_boundary
                && confidence >= D::new(8, 1)
                && cost <= reserve;
            env["snapshot_id"] = json!(snapshot);
            env["strategy_version"] = json!(strategy);
            env["received_at_ms"] = json!(now);
            env["expires_at_ms"] = json!((now + 5000).min(next_boundary));
            env["usable"] = json!(usable);
            env["state"] = json!(if usable {
                "PROPOSAL"
            } else {
                "REJECTED_STATE_CONFIDENCE_OR_COST"
            });
            env["authority"] = json!("PROPOSALS_ONLY");
            env["roundtrip_ms"] = json!(started.elapsed().as_secs_f64() * 1000.);
            env["input"] = input;
            let mut tx = a.db.begin().await?;
            sqlx::query("UPDATE studio.ai_budget SET reserved=reserved-$1::numeric,spent=spent+$2::numeric,failures=0,enabled=enabled AND (spent+$2::numeric<=budget) AND ($2::numeric<=$1::numeric) WHERE id=1").bind(reserve.to_string()).bind(cost.to_string()).execute(&mut *tx).await?;
            sqlx::query("UPDATE studio.decisions SET payload=$1 WHERE id=$2")
                .bind(&env)
                .bind(call_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(env)
        }
        Err(_) => {
            let mut tx = a.db.begin().await?;
            sqlx::query("UPDATE studio.ai_budget SET failures=failures+1,blocked_until=CASE WHEN failures+1>=3 THEN $1 ELSE blocked_until END WHERE id=1").bind(now_ms()+300000).execute(&mut *tx).await?;
            sqlx::query("UPDATE studio.decisions SET payload=payload || $1 WHERE id=$2").bind(json!({"state":"UNKNOWN_COST_OR_INVALID_RESPONSE","error":"Jev timeout, providerfout of ongeldige output. Kostenreserve blijft staan; geen nieuwe blootstelling.","roundtrip_ms":started.elapsed().as_secs_f64()*1000.})).bind(call_id).execute(&mut *tx).await?;
            tx.commit().await?;
            bail!("Jev gaf geen geldig, tijdig antwoord. Kostenreserve blijft staan.")
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> Value {
        json!({"id":"fixture-1","model":"typesafe/jev-1.13-20260917","answers":{"action":{"type":"choice","choice":"HOLD","confidence":0.9,"probabilities":{"HOLD":0.9,"CONSIDER_ENTRY":0.05,"REDUCE":0.05}}},"usage":{"cost":0.00002}})
    }
    #[test]
    fn provider_shape() {
        assert!(parse_decision(&valid()).is_ok());
    }
    #[test]
    fn unpinned_model_fails() {
        let mut v = valid();
        v["model"] = json!("latest");
        assert!(parse_decision(&v).is_err());
    }
    #[test]
    fn missing_cost_fails() {
        let mut v = valid();
        v["usage"] = Value::Null;
        assert!(parse_decision(&v).is_err());
    }
    #[test]
    fn invented_action_fails() {
        let mut v = valid();
        v["answers"]["action"]["choice"] = json!("RAISE_LIMITS");
        assert!(parse_decision(&v).is_err());
    }
    #[test]
    fn missing_probability_fails() {
        let mut v = valid();
        v["answers"]["action"]["probabilities"] = json!({"HOLD":1});
        assert!(parse_decision(&v).is_err());
    }
}
