//! Separate private adapter, never linked by the PAPER_LOCAL terminal binary.
#[path = "../exchange.rs"]
#[allow(dead_code)]
mod exchange;
#[path = "../secrets.rs"]
mod secrets;
use anyhow::{Result, ensure};
#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("TLS-provider"))?;
    let mode = std::env::var("EXECUTION_MODE").unwrap_or_default();
    ensure!(
        mode == "OKX_DEMO",
        "Onboardingtool vereist expliciet OKX_DEMO; LIVE blijft uit"
    );
    let credentials = serde_json::from_str(&secrets::read("okx_demo").await?)?;
    let adapter = exchange::Okx::new(exchange::Mode::OkxDemo, credentials)?;
    let mut snapshot = adapter.snapshot().await?;
    snapshot["private_channels"] = adapter.private_probe().await?;
    let output = std::env::var("ONBOARDING_OUTPUT")
        .unwrap_or_else(|_| "/onboarding/demo-snapshot.json".into());
    tokio::fs::write(output, serde_json::to_vec_pretty(&snapshot)?).await?;
    println!(
        "Demo-accountgegevens gecontroleerd en uitsluitend lokaal opgeslagen. Geen order verzonden. Account/limietbevestiging en technische acceptatie blijven vereist."
    );
    Ok(())
}
