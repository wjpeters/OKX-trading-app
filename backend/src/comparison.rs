//! Compare recorded forward deployments. Never sends an AI request or retrospectively invents decisions.
use crate::{
    App,
    risk::{Bot, Paper},
    studio::{graph_plan, reply},
};
use anyhow::{Result, ensure};
use axum::{Json, Router, extract::State, response::Response, routing::post};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;
pub fn router() -> Router<App> {
    Router::new().route("/api/research/compare", post(compare))
}
fn signature(graph: &Value) -> Result<(Vec<Value>, usize)> {
    let order = graph_plan(graph)?;
    let nodes = graph["nodes"].as_array().unwrap();
    let edges = graph["edges"].as_array().unwrap();
    ensure!(
        edges.len() + 1 == nodes.len(),
        "Vergelijking vereist een enkele strategie-route"
    );
    for id in &order {
        ensure!(
            edges.iter().filter(|e| e["source"] == *id).count() <= 1
                && edges.iter().filter(|e| e["target"] == *id).count() <= 1,
            "Vergelijking van vertakte plannen niet gevalideerd"
        );
    }
    let mut jev = 0;
    let mut sig = vec![];
    for id in order {
        let n = nodes.iter().find(|n| n["id"] == id).unwrap();
        if n["kind"] == "jev" {
            jev += 1;
        } else {
            sig.push(json!({"kind":n["kind"],"params":n["params"]}));
        }
    }
    Ok((sig, jev))
}
fn matched(a: &Bot, b: &Bot, ga: &Value, gb: &Value) -> Result<()> {
    ensure!(
        a.id != b.id && a.instrument == b.instrument,
        "Twee verschillende bots op hetzelfde instrument vereist"
    );
    let mut la = a.limits.clone();
    let mut lb = b.limits.clone();
    la.ai_budget = rust_decimal::Decimal::ZERO;
    lb.ai_budget = rust_decimal::Decimal::ZERO;
    ensure!(
        la == lb
            && a.order_eur == b.order_eur
            && a.stop_bps == b.stop_bps
            && a.target_bps == b.target_bps,
        "Financiële botlimieten en orderinstellingen moeten gelijk zijn"
    );
    let (sa, na) = signature(ga)?;
    let (sb, nb) = signature(gb)?;
    ensure!(
        sa == sb && na == 1 && nb == 0,
        "Eerste bot moet exact dezelfde strategie plus één Jev-filter zijn"
    );
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    with_jev: String,
    without_jev: String,
}
async fn compare(State(a): State<App>, Json(r): Json<Request>) -> Response {
    reply(async {
        let value:Value=sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1").fetch_one(&a.db).await?;
        let p:Paper=serde_json::from_value(value)?;
        let mut configs=vec![];let mut starts=vec![];let mut bots=vec![];
        for id in [&r.with_jev,&r.without_jev] {
            let b=p.bots.get(id).ok_or_else(||anyhow::anyhow!("Bot ontbreekt"))?;ensure!(id!="manual","Selecteer strategie-bots");
            let row=sqlx::query("SELECT d.payload,s.created_ms FROM studio.deployments s JOIN studio.documents d ON d.kind='graph' AND d.name=s.name AND d.version=s.version WHERE s.bot_id=$1").bind(id).fetch_one(&a.db).await?;
            configs.push(row.get::<Value,_>("payload"));starts.push(row.get::<i64,_>("created_ms"));bots.push(b);
        }
        matched(bots[0],bots[1],&configs[0],&configs[1])?;
        let start=*starts.iter().max().unwrap();ensure!((starts[0]-starts[1]).abs()<=60000,"Deploy beide testbots binnen dezelfde minuut");
        for b in &bots {ensure!(!p.orders.values().any(|o|o.intent.bot_id==b.id&&o.created_ms<start),"Gebruik nieuwe bots zonder oudere handelsgeschiedenis voor een zuivere vergelijking");}
        let mut metrics=vec![];
        for b in bots {
            let costs=sqlx::query("SELECT count(*) AS decisions,coalesce(sum((payload->>'cost_usd')::numeric),0)::text AS spent,coalesce(sum(CASE WHEN payload->>'cost_usd' IS NULL THEN (payload->>'reserved_usd')::numeric ELSE 0 END),0)::text AS reserved FROM studio.decisions WHERE payload->'input'->>'bot_id'=$1 AND created_ms>=$2").bind(&b.id).bind(start).fetch_one(&a.db).await?;
            let fills:Vec<_>=p.fills.iter().filter(|f|p.orders.get(&f.order_id).is_some_and(|o|o.intent.bot_id==b.id)).collect();
            let turnover:rust_decimal::Decimal=fills.iter().map(|f|f.quantity*f.price).sum();
            metrics.push(json!({"bot":b.id,"name":b.name,"strategy":b.version,"running":b.running,"equity_eur":b.ledger.equity,"net_pnl_eur_before_ai":b.ledger.equity-b.ledger.baseline,"fees_eur":b.ledger.fees,"drawdown_eur":b.ledger.drawdown,"turnover_eur":turnover,"fills":fills.len(),"ai_decisions":costs.get::<i64,_>("decisions"),"ai_spent_usd":costs.get::<String,_>("spent"),"ai_reserved_usd":costs.get::<String,_>("reserved"),"last_signal":b.last_signal}));
        }
        Ok(json!({"mode":"FORWARD_PAPER_COMPARISON","start_ms":start,"checked_same_strategy_and_limits":true,"metrics":metrics,"limitations":"Observed forward data only. Shared account limits/liquidity can influence both bots. EUR trading P&L and USD AI costs shown separately, without an invented FX conversion. No profitability or causal claim."}))
    }.await)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph(jev: bool) -> Value {
        let mut ns = vec!["market", "sma_cross"];
        if jev {
            ns.push("jev");
        }
        ns.extend(["intent", "risk_gate"]);
        json!({"schema":1,"nodes":ns.iter().enumerate().map(|(i,k)|json!({"id":i.to_string(),"kind":k,"params":{}})).collect::<Vec<_>>(),"edges":(1..ns.len()).map(|i|json!({"source":(i-1).to_string(),"target":i.to_string()})).collect::<Vec<_>>()})
    }
    #[test]
    fn only_ai_filter_removed_from_comparison() {
        let (a, n) = signature(&graph(true)).unwrap();
        let (b, m) = signature(&graph(false)).unwrap();
        assert_eq!(a, b);
        assert_eq!((n, m), (1, 0));
    }
    #[test]
    fn parameter_change_is_not_same_strategy() {
        let a = graph(true);
        let mut b = graph(false);
        b["nodes"][1]["params"] = json!({"fast":2});
        assert_ne!(signature(&a).unwrap().0, signature(&b).unwrap().0);
    }
}
