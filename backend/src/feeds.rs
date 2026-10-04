use crate::{App, model::now_ms};
use anyhow::{Result, bail};
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
fn decimal(v: &Value) -> bool {
    v.as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .is_some_and(|n| n >= Decimal::ZERO)
}
fn timestamp(v: &Value) -> bool {
    v.as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .is_some_and(|n| n > 0)
}
fn valid(channel: &str, d: &Value) -> bool {
    if !timestamp(&d["ts"]) {
        return false;
    }
    match channel {
        "tickers" => ["last", "open24h", "high24h", "low24h", "vol24h"]
            .iter()
            .all(|k| decimal(&d[k])),
        "books5" | "books" => ["asks", "bids"].iter().all(|k| {
            d[k].as_array().is_some_and(|levels| {
                levels.len() <= if channel == "books" { 400 } else { 5 }
                    && levels.iter().all(|r| {
                        r.as_array()
                            .is_some_and(|a| a.len() >= 2 && decimal(&a[0]) && decimal(&a[1]))
                    })
            })
        }),
        "trades" => {
            decimal(&d["px"])
                && decimal(&d["sz"])
                && d["tradeId"].is_string()
                && ["buy", "sell"].iter().any(|s| d["side"] == *s)
        }
        _ => false,
    }
}
pub async fn run(app: App) {
    loop {
        match catalogue(&app).await {
            Ok(()) => break,
            Err(e) => {
                app.market.write().await.feed_error = Some(e.to_string());
                tracing::warn!(error=%e,"Catalogue unavailable");
                tokio::time::sleep(Duration::from_secs(15)).await;
            }
        }
    }
    let mut failures = 0u32;
    loop {
        let start = Instant::now();
        let result = connection(&app).await;
        {
            let mut m = app.market.write().await;
            m.feed_connected = false;
            m.reconnects += 1;
            m.feed_error = Some(
                result
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Verbinding gesloten".into()),
            );
        }
        failures = if start.elapsed() > Duration::from_secs(60) {
            1
        } else {
            (failures + 1).min(6)
        };
        // Time-derived jitter is not cryptographic and has no security role.
        tokio::time::sleep(Duration::from_millis(
            (1u64 << failures) * 500 + (now_ms() as u64 % 700),
        ))
        .await;
    }
}
async fn catalogue(app: &App) -> Result<()> {
    let local = now_ms();
    let time = app.public_get("/api/v5/public/time", &[]).await?;
    let end = now_ms();
    let ts = time["data"][0]["ts"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Geen OKX tijd"))?
        .parse::<i64>()?;
    let all = app
        .public_get("/api/v5/public/instruments", &[("instType", "SPOT")])
        .await?;
    let preferred = [
        "BTC-EUR", "ETH-EUR", "SOL-EUR", "XRP-EUR", "DOGE-EUR", "ADA-EUR", "AVAX-EUR", "LINK-EUR",
        "BTC-USDC", "ETH-USDC", "SOL-USDC", "SUI-USDC",
    ];
    let instruments: Vec<_> = preferred
        .iter()
        .filter_map(|id| {
            all["data"]
                .as_array()?
                .iter()
                .find(|v| {
                    v["instId"] == *id
                        && v["state"] == "live"
                        && decimal(&v["tickSz"])
                        && decimal(&v["lotSz"])
                        && decimal(&v["minSz"])
                })
                .cloned()
        })
        .collect();
    if instruments.is_empty() {
        bail!("Geen aangevraagde spotinstrumenten beschikbaar; geen fallback naar andere regio");
    }
    let mut m = app.market.write().await;
    m.instruments = instruments;
    m.metadata_received_ms = Some(now_ms());
    m.clock_offset_ms = Some(ts - (local + end) / 2);
    m.clock_rtt_ms = Some(end - local);
    Ok(())
}
async fn connection(app: &App) -> Result<()> {
    let (mut socket, _) = tokio::time::timeout(
        Duration::from_secs(15),
        connect_async("wss://wseea.okx.com/ws/v5/public"),
    )
    .await??;
    let ids: Vec<String> = app
        .market
        .read()
        .await
        .instruments
        .iter()
        .filter_map(|v| v["instId"].as_str().map(str::to_owned))
        .collect();
    let args: Vec<_> = ids
        .iter()
        .flat_map(|id| {
            ["tickers", "books", "trades"]
                .into_iter()
                .map(move |ch| json!({"channel":ch,"instId":id}))
        })
        .collect();
    socket
        .send(Message::Text(
            json!({"op":"subscribe","args":args}).to_string().into(),
        ))
        .await?;
    {
        let mut m = app.market.write().await;
        m.feed_error = None;
        m.books.clear();
        m.tickers.clear();
        m.trades.clear();
    }
    let mut books = std::collections::BTreeMap::<String, crate::book::Book>::new();
    loop {
        let incoming = match tokio::time::timeout(Duration::from_secs(20), socket.next()).await {
            Ok(v) => v,
            Err(_) => {
                socket.send(Message::Text("ping".into())).await?;
                tokio::time::timeout(Duration::from_secs(10), socket.next()).await?
            }
        };
        let Some(message) = incoming else {
            bail!("OKX sloot de verbinding");
        };
        let text = match message? {
            Message::Text(s) => s,
            Message::Ping(p) => {
                socket.send(Message::Pong(p)).await?;
                continue;
            }
            Message::Close(_) => bail!("OKX sloot de verbinding"),
            _ => continue,
        };
        if text == "pong" {
            continue;
        }
        if text.len() > 2_000_000 {
            bail!("WS bericht te groot");
        }
        let start = Instant::now();
        let received = now_ms();
        let v: Value = serde_json::from_str(&text)?;
        if v["event"] == "error" {
            bail!("OKX WS fout {}", v["code"]);
        }
        let Some(data) = v["data"].as_array() else {
            continue;
        };
        let id = v["arg"]["instId"].as_str().unwrap_or("");
        let channel = v["arg"]["channel"].as_str().unwrap_or("");
        if !ids.iter().any(|s| s == id) {
            bail!("Onbekend WS instrument");
        }
        let mut market = app.market.write().await;
        for raw in data {
            if !valid(channel, raw) {
                bail!("Ongeldig {} schema", channel);
            }
            let mut d = raw.clone();
            d["received_at_ms"] = json!(received);
            // Out-of-order snapshots never replace newer state; books5 carries complete snapshots.
            let stamp = |v: &Value| {
                v["ts"]
                    .as_str()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
            };
            match channel {
                "tickers" => {
                    if market
                        .tickers
                        .get(id)
                        .is_none_or(|old| stamp(&d) >= stamp(old))
                    {
                        market.tickers.insert(id.into(), d);
                    }
                }
                "books" => {
                    let book = books.entry(id.into()).or_default();
                    let snapshot = book.apply(v["action"].as_str().unwrap_or(""), raw, received)?;
                    market.books.insert(id.into(), snapshot);
                }
                "books5" => {
                    if market
                        .books
                        .get(id)
                        .is_none_or(|old| stamp(&d) >= stamp(old))
                    {
                        market.books.insert(id.into(), d);
                    }
                }
                "trades" => {
                    let trades = market.trades.entry(id.into()).or_insert_with(VecDeque::new);
                    if !trades.iter().any(|t| t["tradeId"] == d["tradeId"]) {
                        trades.push_front(d);
                        if trades.len() > 100 {
                            trades.pop_back();
                        }
                    }
                }
                _ => bail!("Onbekend kanaal"),
            }
        }
        market.feed_connected = true;
        market.last_message_ms = Some(received);
        market.validated_messages = market.validated_messages.saturating_add(1);
        drop(market);
        let mut metrics = app.metrics.lock().await;
        metrics.push_back(start.elapsed().as_secs_f64() * 1000.0);
        if metrics.len() > 2048 {
            metrics.pop_front();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bad_books_rejected() {
        assert!(!valid(
            "books5",
            &json!({"ts":"1","asks":[["NaN","1"]],"bids":[]})
        ));
    }
    #[test]
    fn snapshots_accept_empty_book() {
        assert!(valid("books5", &json!({"ts":"1","asks":[],"bids":[]})));
    }
}

/// Public clock sampling is separate from the deterministic order route.
pub async fn refresh_clock(app: App) {
    let mut cycles = 0u32;
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        cycles += 1;
        if cycles % 5 == 0 {
            if catalogue(&app).await.is_err() {
                app.market.write().await.metadata_received_ms = None;
            }
        }
        let start = now_ms();
        match app.public_get("/api/v5/public/time", &[]).await {
            Ok(v) => {
                let end = now_ms();
                let ts = v["data"][0]["ts"]
                    .as_str()
                    .and_then(|s| s.parse::<i64>().ok());
                let mut m = app.market.write().await;
                m.clock_offset_ms = ts.map(|ts| ts - (start + end) / 2);
                m.clock_rtt_ms = Some(end - start);
            }
            Err(_) => {
                let mut m = app.market.write().await;
                m.clock_offset_ms = None;
                m.clock_rtt_ms = None;
            }
        }
    }
}
