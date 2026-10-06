//! Validated account balances, independent of permission to send an order.
use crate::order_types::*;
use anyhow::{Result, ensure};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub currency: String,
    pub total: D,
    pub available: Option<D>,
    pub frozen: Option<D>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallet {
    pub uid: String,
    pub mode: ExecutionMode,
    pub received_ms: i64,
    pub trading_received_ms: i64,
    pub trade_permission: bool,
    pub trading: Vec<Asset>,
    pub funding: Option<Vec<Asset>>,
    pub funding_reason: String,
}
fn decimal(v: &Value) -> Result<D> {
    v.as_str()
        .ok_or_else(|| anyhow::anyhow!("Saldo heeft geen decimaalformaat"))?
        .parse()
        .map_err(Into::into)
}
fn optional(v: &Value) -> Result<Option<D>> {
    if v.is_null() || v == "" {
        Ok(None)
    } else {
        Ok(Some(decimal(v)?))
    }
}
pub fn assets(items: &[Value], funding: bool) -> Result<Vec<Asset>> {
    ensure!(items.len() <= 1000, "Te veel saldoregels");
    let mut seen = BTreeSet::new();
    let mut result = vec![];
    for row in items {
        let c = row["ccy"]
            .as_str()
            .filter(|c| {
                !c.is_empty() && c.len() <= 30 && c.bytes().all(|b| b.is_ascii_alphanumeric())
            })
            .ok_or_else(|| anyhow::anyhow!("Ongeldige saldovaluta"))?;
        ensure!(seen.insert(c.to_string()), "Dubbele saldovaluta");
        let total = decimal(&row[if funding { "bal" } else { "cashBal" }])?;
        let available = optional(&row["availBal"])?;
        let frozen = optional(&row["frozenBal"])?;
        result.push(Asset {
            currency: c.into(),
            total,
            available,
            frozen,
        });
    }
    result.sort_by_key(|a| {
        (
            match a.currency.as_str() {
                "EUR" => 0,
                "BTC" => 1,
                "USDT" => 2,
                _ => 3,
            },
            a.currency.clone(),
        )
    });
    Ok(result)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeRequest {
    pub mode: ExecutionMode,
    pub instrument: String,
    pub side: Direction,
    pub percent: u16,
    pub order: OrderSpec,
}
#[derive(Serialize)]
pub struct Sized {
    pub account_uid: String,
    pub currency: String,
    pub available: D,
    pub selected: D,
    pub quantity: D,
    pub percent: u16,
    pub price_bound: D,
    pub minimum: D,
    pub below_minimum: bool,
    pub wallet_received_ms: i64,
}
pub fn size(
    r: &SizeRequest,
    w: &Wallet,
    rules: &MarketRules,
    price: D,
    fee: Option<D>,
    reserved: D,
    now: i64,
) -> Result<Sized> {
    ensure!(
        r.mode == w.mode && (0..=15000).contains(&(now - w.trading_received_ms)),
        "Accountsaldo ontbreekt of is verouderd"
    );
    ensure!(
        r.instrument == format!("{}-{}", rules.base, rules.quote) && ["EUR","USDC"].contains(&rules.quote.as_str()),
        "Geen ondersteunde spotmarkt"
    );
    ensure!(
        r.percent <= 100 && rules.lot > D::ZERO && rules.minimum > D::ZERO && reserved >= D::ZERO,
        "Ongeldig percentage of instrumentregels"
    );
    let currency = if r.side == Direction::Buy {
        &rules.quote
    } else {
        &rules.base
    };
    // Missing assets are a known zero only in a successfully fetched complete wallet.
    let free = match w.trading.iter().find(|a| &a.currency == currency) {
        Some(a) => a
            .available
            .ok_or_else(|| anyhow::anyhow!("Beschikbaar saldo ontbreekt voor deze valuta"))?,
        None => D::ZERO,
    };
    ensure!(free >= D::ZERO, "Geen geleend saldo voor percentageorders");
    let available = (free - reserved).max(D::ZERO);
    let selected = available * D::from(r.percent) / D::from(100);
    let bound = if r.side == Direction::Sell {
        price
    } else {
        match &r.order {
            OrderSpec::Market { slippage_bps } => {
                ensure!(*slippage_bps <= 500, "Ongeldige slippage");
                price * (D::ONE + D::from(*slippage_bps) / D::from(10000))
            }
            OrderSpec::Limit { price, .. } => *price,
            OrderSpec::Trigger {
                execution: ExecutionPrice::Limit { price, .. },
                ..
            } => *price,
            OrderSpec::Iceberg { price_limit, .. } | OrderSpec::Twap { price_limit, .. } => {
                *price_limit
            }
            _ => anyhow::bail!("Kies een maximale koopprijs voor percentageorders"),
        }
    };
    positive(bound)?;
    let raw = if r.side == Direction::Buy {
        let fee = fee.ok_or_else(|| {
            anyhow::anyhow!("Actuele accountfee ontbreekt; wacht op de orderverbinding")
        })?;
        ensure!(
            fee >= D::ZERO && fee <= D::new(1, 1),
            "Ongeldige accountfee"
        );
        selected / (bound * (D::ONE + fee))
    } else {
        selected
    };
    let quantity = (raw / rules.lot).floor() * rules.lot;
    Ok(Sized {
        account_uid: w.uid.clone(),
        currency: currency.clone(),
        available,
        selected,
        quantity,
        percent: r.percent,
        price_bound: bound,
        minimum: rules.minimum,
        below_minimum: quantity < rules.minimum,
        wallet_received_ms: w.trading_received_ms,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> D {
        s.parse().unwrap()
    }
    fn fixture() -> (SizeRequest, Wallet, MarketRules) {
        (
            SizeRequest {
                mode: ExecutionMode::Live,
                instrument: "BTC-EUR".into(),
                side: Direction::Sell,
                percent: 50,
                order: OrderSpec::Market { slippage_bps: 100 },
            },
            Wallet {
                uid: "fixture".into(),
                mode: ExecutionMode::Live,
                received_ms: 10000,
                trading_received_ms: 10000,
                trade_permission: false,
                trading: vec![
                    Asset {
                        currency: "BTC".into(),
                        total: d("1"),
                        available: Some(d("0.4")),
                        frozen: Some(d("0.6")),
                    },
                    Asset {
                        currency: "EUR".into(),
                        total: d("100"),
                        available: Some(d("100")),
                        frozen: Some(D::ZERO),
                    },
                ],
                funding: None,
                funding_reason: String::new(),
            },
            MarketRules {
                tick: d("0.1"),
                lot: d("0.0001"),
                minimum: d("0.0001"),
                base: "BTC".into(),
                quote: "EUR".into(),
            },
        )
    }
    #[test]
    fn sell_sizes_free_base_after_claims_without_trade_permission() {
        let (r, w, m) = fixture();
        let s = size(&r, &w, &m, d("50000"), None, d("0.1"), 10000).unwrap();
        assert_eq!(s.quantity, d("0.15"));
        assert_eq!(s.currency, "BTC");
    }
    #[test]
    fn buy_includes_fee_slippage_and_floors_lots() {
        let (mut r, w, m) = fixture();
        r.side = Direction::Buy;
        r.percent = 100;
        let s = size(&r, &w, &m, d("50000"), Some(d("0.001")), D::ZERO, 10000).unwrap();
        assert_eq!(s.quantity, d("0.0019"));
        assert!(s.quantity * s.price_bound * (D::ONE + d("0.001")) <= d("100"));
    }
    #[test]
    fn rejects_stale_wrong_mode_unbounded_and_missing_fee() {
        let (mut r, w, m) = fixture();
        assert!(size(&r, &w, &m, d("50000"), None, D::ZERO, 30000).is_err());
        r.mode = ExecutionMode::OkxDemo;
        assert!(size(&r, &w, &m, d("50000"), None, D::ZERO, 10000).is_err());
        r.mode = ExecutionMode::Live;
        r.side = Direction::Buy;
        assert!(size(&r, &w, &m, d("50000"), None, D::ZERO, 10000).is_err());
        r.order = OrderSpec::Trigger {
            trigger: d("50000"),
            execution: ExecutionPrice::Market,
        };
        assert!(size(&r, &w, &m, d("50000"), Some(D::ZERO), D::ZERO, 10000).is_err());
    }
    #[test]
    fn zero_is_preview_only_and_missing_available_is_unknown() {
        let (mut r, mut w, m) = fixture();
        r.percent = 0;
        assert!(
            size(&r, &w, &m, d("50000"), None, D::ZERO, 10000)
                .unwrap()
                .below_minimum
        );
        w.trading[0].available = None;
        assert!(size(&r, &w, &m, d("50000"), None, D::ZERO, 10000).is_err());
    }
    #[test]
    fn parses_all_assets_and_rejects_duplicate_or_malformed() {
        let v = serde_json::json!([{"ccy":"USDT","cashBal":"3","availBal":"2","frozenBal":"1"},{"ccy":"BTC","cashBal":"0.1","availBal":"","frozenBal":""}]);
        let a = assets(v.as_array().unwrap(), false).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].available, None);
        assert!(assets(&[v[0].clone(), v[0].clone()], false).is_err());
        assert!(assets(&[serde_json::json!({"ccy":"EUR","cashBal":3})], false).is_err());
    }
}
