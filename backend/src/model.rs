use anyhow::{Result, anyhow, bail};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub ts: i64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub closed: bool,
}
impl Candle {
    pub fn parse(v: &Value) -> Result<Self> {
        let a = v
            .as_array()
            .ok_or_else(|| anyhow!("Candle is geen array"))?;
        if a.len() != 9 {
            bail!("Onverwacht candleschema");
        }
        let s = |i: usize| {
            a[i].as_str()
                .ok_or_else(|| anyhow!("Candleveld is geen string"))
        };
        let c = Self {
            ts: s(0)?.parse()?,
            open: Decimal::from_str(s(1)?)?,
            high: Decimal::from_str(s(2)?)?,
            low: Decimal::from_str(s(3)?)?,
            close: Decimal::from_str(s(4)?)?,
            volume: Decimal::from_str(s(5)?)?,
            closed: match s(8)? {
                "1" => true,
                "0" => false,
                _ => bail!("Ongeldige bevestiging"),
            },
        };
        if c.ts <= 0
            || c.low <= Decimal::ZERO
            || c.high < c.low
            || c.open < c.low
            || c.open > c.high
            || c.close < c.low
            || c.close > c.high
            || c.volume < Decimal::ZERO
        {
            bail!("Ongeldige OHLCV");
        }
        Ok(c)
    }
}
pub fn interval(bar: &str) -> Option<i64> {
    match bar {
        "1m" => Some(60_000),
        "5m" => Some(300_000),
        "15m" => Some(900_000),
        "1H" => Some(3_600_000),
        "4H" => Some(14_400_000),
        "1Dutc" => Some(86_400_000),
        _ => None,
    }
}
pub fn normalize(values: &[Value], step: i64) -> Result<(Vec<Candle>, usize)> {
    let mut map = std::collections::BTreeMap::new();
    for v in values {
        let c = Candle::parse(v)?;
        if c.ts % step != 0 {
            bail!("Candle niet op timeframegrens");
        }
        map.insert(c.ts, c);
    }
    let candles: Vec<_> = map.into_values().collect();
    let gaps = candles
        .windows(2)
        .filter(|w| w[1].ts - w[0].ts != step)
        .count();
    Ok((candles, gaps))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn row(ts: &str) -> Value {
        json!([ts, "10", "12", "9", "11", "2", "20", "20", "1"])
    }
    #[test]
    fn exact_decimals() {
        let mut v = row("60000");
        v[1] = json!("10.123456789123456789");
        assert_eq!(
            Candle::parse(&v).unwrap().open.to_string(),
            "10.123456789123456789"
        );
    }
    #[test]
    fn gaps_are_not_filled() {
        let (cs, g) = normalize(&[row("180000"), row("60000"), row("60000")], 60000).unwrap();
        assert_eq!(cs.len(), 2);
        assert_eq!(g, 1);
    }
    #[test]
    fn bad_prices_fail() {
        let mut v = row("60000");
        v[4] = json!("99");
        assert!(Candle::parse(&v).is_err());
    }
    #[test]
    fn malformed_confirm_fails() {
        let mut v = row("60000");
        v[8] = json!("2");
        assert!(Candle::parse(&v).is_err());
    }
    #[test]
    fn synthetic_timeframes_rejected() {
        assert!(interval("HeikinAshi").is_none());
    }
}
