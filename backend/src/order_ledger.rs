//! Pure execution accounting. Exchange confirmations, never ACKs, change inventory.
use crate::order_types::*;
use anyhow::{Result, ensure};
use rust_decimal::Decimal as D;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Fill {
    pub bill_id: String,
    pub exchange_id: String,
    pub instrument: String,
    pub side: Direction,
    pub quantity: D,
    pub price: D,
    pub fee: D,
    pub fee_currency: String,
    pub fill_ms: i64,
}
fn dec(v: &Value) -> Result<D> {
    v.as_str()
        .ok_or_else(|| anyhow::anyhow!("Decimaalveld ontbreekt"))?
        .parse()
        .map_err(Into::into)
}
impl Fill {
    pub fn parse(v: &Value) -> Result<Self> {
        let text = |key: &str| -> Result<String> {
            Ok(v[key]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 100)
                .ok_or_else(|| anyhow::anyhow!("Fill-identiteit ontbreekt: {key}"))?
                .into())
        };
        let f = Self {
            bill_id: text("billId")?,
            exchange_id: text("ordId")?,
            instrument: text("instId")?,
            side: serde_json::from_value(v["side"].clone())?,
            quantity: dec(&v["fillSz"])?,
            price: dec(&v["fillPx"])?,
            fee: dec(&v["fee"])?,
            fee_currency: text("feeCcy")?,
            fill_ms: text("ts")?.parse()?,
        };
        positive(f.quantity)?;
        positive(f.price)?;
        ensure!(f.fee.abs() <= f.quantity * f.price, "Onverklaarde fill-fee");
        Ok(f)
    }
    pub fn inventory(&self, base: &str) -> D {
        let gross = if self.side == Direction::Buy {
            self.quantity
        } else {
            -self.quantity
        };
        gross
            + if self.fee_currency == base {
                self.fee
            } else {
                D::ZERO
            }
    }
    pub fn postings(&self, base: &str, quote: &str) -> BTreeMap<String, D> {
        let sign = if self.side == Direction::Buy {
            D::ONE
        } else {
            -D::ONE
        };
        let mut p = BTreeMap::from([
            (base.into(), sign * self.quantity),
            (quote.into(), -sign * self.quantity * self.price),
        ]);
        *p.entry(self.fee_currency.clone()).or_default() += self.fee;
        p
    }
}
pub fn protection_quantity(net_filled: D, committed: D, lot: D, minimum: D) -> Result<(D, D)> {
    ensure!(
        net_filled >= D::ZERO && committed >= D::ZERO && lot > D::ZERO && minimum > D::ZERO,
        "Ongeldige beschermingsadministratie"
    );
    ensure!(
        committed <= net_filled,
        "Beschermingsorders overschrijden werkelijk verworven inventaris"
    );
    let delta = net_filled - committed;
    let rounded = (delta / lot).floor() * lot;
    Ok((if rounded >= minimum { rounded } else { D::ZERO }, delta))
}
pub fn regular_state(v: &Value) -> Result<&'static str> {
    match v["state"].as_str() {
        Some("filled") => Ok("FILLED"),
        Some("canceled" | "mmp_canceled") => Ok("CANCELED"),
        Some("partially_filled") => Ok("PARTIAL_FILL"),
        Some("live") => Ok("ACCEPTED"),
        _ => anyhow::bail!("Onbekende OKX-orderstatus"),
    }
}
pub fn algorithm_state(
    v: &Value,
    quantity: D,
    filled: D,
    children_terminal: bool,
    has_children: bool,
) -> &'static str {
    if v["state"] == "canceled" && children_terminal {
        "CANCELED"
    } else if v["state"] == "order_failed" {
        "REJECTED"
    } else if filled >= quantity && has_children && children_terminal {
        "FILLED"
    } else if filled > D::ZERO {
        "PARTIAL_FILL"
    } else {
        "ACCEPTED"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fill(fee: &str, ccy: &str) -> Fill {
        Fill::parse(&json!({"billId":"123","ordId":"456","instId":"BTC-EUR","side":"buy","fillSz":"1","fillPx":"100","fee":fee,"feeCcy":ccy,"ts":"10000"})).unwrap()
    }
    #[test]
    fn base_fee_changes_protectable_quantity() {
        let f = fill("-0.001", "BTC");
        assert_eq!(f.inventory("BTC"), D::new(999, 3));
        assert_eq!(f.postings("BTC", "EUR")["EUR"], D::from(-100));
    }
    #[test]
    fn quote_fee_and_rebate_are_booked_in_correct_currency() {
        let f = fill("-0.1", "EUR");
        assert_eq!(f.inventory("BTC"), D::ONE);
        assert_eq!(f.postings("BTC", "EUR")["EUR"], D::new(-1001, 1));
        let f = fill("0.1", "EUR");
        assert_eq!(f.postings("BTC", "EUR")["EUR"], D::new(-999, 1));
    }
    #[test]
    fn fee_adjusted_partial_fill_dust_is_never_oversold() {
        let (q, d) =
            protection_quantity(D::new(999, 3), D::ZERO, D::new(1, 2), D::new(1, 2)).unwrap();
        assert_eq!(q, D::new(99, 2));
        assert_eq!(d, D::new(999, 3));
        let (q, _) =
            protection_quantity(D::new(999, 3), D::new(99, 2), D::new(1, 2), D::new(1, 2)).unwrap();
        assert_eq!(q, D::ZERO);
    }
    #[test]
    fn duplicate_fill_identity_can_only_post_once() {
        let f = fill("-0.1", "EUR");
        let mut ledger = BTreeMap::new();
        ledger.entry(f.bill_id.clone()).or_insert(f.clone());
        ledger.entry(f.bill_id.clone()).or_insert(f);
        assert_eq!(ledger.len(), 1);
    }
    #[test]
    fn effective_trigger_is_not_a_fill() {
        assert_eq!(
            algorithm_state(&json!({"state":"effective"}), D::ONE, D::ZERO, true, false),
            "ACCEPTED"
        );
        assert_eq!(
            algorithm_state(&json!({"state":"effective"}), D::ONE, D::ONE, true, true),
            "FILLED"
        );
    }
    #[test]
    fn cancel_with_partial_children_is_not_final_before_children() {
        assert_eq!(
            algorithm_state(
                &json!({"state":"canceled"}),
                D::ONE,
                D::new(1, 1),
                false,
                true
            ),
            "PARTIAL_FILL"
        );
        assert_eq!(
            algorithm_state(
                &json!({"state":"canceled"}),
                D::ONE,
                D::new(1, 1),
                true,
                true
            ),
            "CANCELED"
        );
    }
    #[test]
    fn impossible_protection_and_unknown_state_fail() {
        assert!(protection_quantity(D::ONE, D::from(2), D::ONE, D::ONE).is_err());
        assert!(regular_state(&json!({"state":"banana"})).is_err());
    }
}
