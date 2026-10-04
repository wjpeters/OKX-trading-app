//! Read-only operational evidence and explicit release gates.
use crate::{App, model::now_ms, studio::reply};
use anyhow::Result;
use axum::{Router, extract::State, response::Response, routing::get};
use serde_json::{Value, json};
use std::time::Duration;
pub fn router() -> Router<App> {
    Router::new()
        .route("/api/operations", get(status))
        .route("/api/metrics", get(metrics))
}
async fn status(State(a): State<App>) -> Response {
    reply(async{let db_bytes:i64=sqlx::query_scalar("SELECT pg_database_size(current_database())").fetch_one(&a.db).await?;let usage=disk_usage().await;let p:Option<Value>=sqlx::query_scalar("SELECT payload FROM paper_v2.account WHERE id=1").fetch_optional(&a.db).await?;let unknown=p.as_ref().and_then(|p|p["orders"].as_object()).map(|os|os.values().filter(|o|o["state"]=="UNKNOWN").count()).unwrap_or(0);let unprotected=p.as_ref().and_then(|p|p["positions"].as_object()).map(|ps|ps.values().filter(|p|p["quantity"].as_str().unwrap_or("0")!="0"&&p["protection_state"]!="ARMED"&&p["protection_state"]!="EXIT_PENDING").count()).unwrap_or(0);let database_pressure=usage.is_none_or(|v|v>=90);Ok(json!({"database_bytes":db_bytes,"archive_disk_used_percent":usage,"storage_alert":database_pressure,"unknown_orders":unknown,"unprotected_or_dust_positions":unprotected,"authentication":{"configured":crate::auth::configured(),"binding":"127.0.0.1 only","session_hours":8},"isolation":{"paper":"No private exchange module imported","pine":"Network disabled; no secrets; per-run process deadline","jev":"Decisions API; proposals only; fixed risk prefilter and final check"},"gates":[{"name":"Lokale paperhandel","state":"AVAILABLE"},{"name":"Jev externe calls","state":if std::path::Path::new("/run/secrets/openrouter_key").is_file(){"BUDGET_CONFIRMATION_REQUIRED"}else{"KEY_REQUIRED"}},{"name":"OKX Demo","state":"ONBOARDING_AND_EXECUTION_ACCEPTANCE_REQUIRED"},{"name":"LIVE","state":"BLOCKED_ACCOUNT_LIMITS_PROTECTION_AND_ACCEPTANCE"},{"name":"Pine intrabar / request.security / strategy.*","state":"COMPATIBILITY_VALIDATION_REQUIRED"},{"name":"DeMARK","state":"LICENSE_REQUIRED"}],"limits":"Loss thresholds are actions, not guarantees. Local protection pauses while host sleeps. No VPS deployment.","server_time_ms":now_ms()}))}.await)
}
async fn metrics(State(a): State<App>) -> String {
    let m = a.market.read().await;
    format!(
        "# TYPE okx_public_connected gauge\nokx_public_connected {}\n# TYPE okx_paper_worker_age_ms gauge\nokx_paper_worker_age_ms {}\n# TYPE okx_feed_reconnects counter\nokx_feed_reconnects {}\n",
        if m.feed_connected { 1 } else { 0 },
        (now_ms() - a.paper_heartbeat.load(std::sync::atomic::Ordering::Relaxed)).max(0),
        m.reconnects
    )
}
pub async fn watch(a: App) {
    loop {
        if let Err(e) = cycle(&a).await {
            tracing::warn!(error=%e,"Storagebewaking niet beschikbaar")
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
async fn disk_usage() -> Option<u32> {
    let output = tokio::time::timeout(
        Duration::from_secs(2),
        tokio::process::Command::new("df")
            .args(["-Pk", "/archives"])
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1)
        .flat_map(str::split_whitespace)
        .find_map(|s| s.strip_suffix('%').and_then(|p| p.parse().ok()))
}
fn storage_pause_reason(used: Option<u32>) -> Option<&'static str> {
    match used {
        None => Some("Opslagmeting niet beschikbaar; nieuwe entries geblokkeerd"),
        Some(n) if n >= 95 => Some("Opslag >=95%; nieuwe entries geblokkeerd"),
        _ => None,
    }
}
async fn cycle(a: &App) -> Result<()> {
    if let Some(reason) = storage_pause_reason(disk_usage().await) {
        let mut tx = a.db.begin().await?;
        if let Ok(mut p) = crate::paper::load(&mut tx).await {
            if !p.ledger.paused {
                p.control("PAUSE_ENTRIES", &crate::paper::quotes(a).await, now_ms())?;
                p.event(now_ms(), "STORAGE_PRESSURE", "account", reason);
                crate::paper::persist(&mut tx, &mut p).await?;
            }
        }
        tx.commit().await?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_storage_and_unavailable_probe_block_entries() {
        assert!(storage_pause_reason(Some(95)).is_some());
        assert!(storage_pause_reason(Some(100)).is_some());
        assert!(storage_pause_reason(None).is_some());
        assert!(storage_pause_reason(Some(94)).is_none());
    }
}
