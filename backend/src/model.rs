use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Datelike, Months, Utc, Weekday};
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
        "3m" => Some(180_000),
        "30m" => Some(1_800_000),
        "2H" => Some(7_200_000),
        "6Hutc" => Some(21_600_000),
        "12Hutc" => Some(43_200_000),
        "1Wutc" => Some(604_800_000),
        "5m" => Some(300_000),
        "15m" => Some(900_000),
        "1H" => Some(3_600_000),
        "4H" => Some(14_400_000),
        "1Dutc" => Some(86_400_000),
        _ => None,
    }
}
/// Calendar periods must never be represented as a fixed 30-day duration.
#[derive(Clone, Copy, Debug)]
pub enum Timeframe {
    Fixed(i64),
    Week,
    CalendarMonths(u32),
}
impl Timeframe {
    pub fn parse(bar: &str) -> Option<Self> {
        match bar {
            "1Wutc" => Some(Self::Week),
            "1Mutc" => Some(Self::CalendarMonths(1)),
            "3Mutc" => Some(Self::CalendarMonths(3)),
            _ => interval(bar).map(Self::Fixed),
        }
    }
    pub fn aligned(self, ts: i64) -> bool {
        match self {
            Self::Fixed(step) => step > 0 && ts.rem_euclid(step) == 0,
            _ => {
                let Some(date) = DateTime::<Utc>::from_timestamp_millis(ts) else {
                    return false;
                };
                if ts.rem_euclid(86_400_000) != 0 {
                    return false;
                }
                match self {
                    Self::Week => date.weekday() == Weekday::Mon,
                    Self::CalendarMonths(n) => date.day() == 1 && date.month0() % n == 0,
                    _ => false,
                }
            }
        }
    }
    pub fn next_open(self, ts: i64) -> Option<i64> {
        match self {
            Self::Fixed(step) => ts.checked_add(step),
            Self::Week => ts.checked_add(604_800_000),
            Self::CalendarMonths(n) => DateTime::<Utc>::from_timestamp_millis(ts)?
                .checked_add_months(Months::new(n))
                .map(|d| d.timestamp_millis()),
        }
    }
    pub fn consecutive(self, a: i64, b: i64) -> bool {
        self.next_open(a) == Some(b)
    }
}
pub fn normalize(values: &[Value], timeframe: Timeframe) -> Result<(Vec<Candle>, usize)> {
    let mut map = std::collections::BTreeMap::new();
    for v in values {
        let c = Candle::parse(v)?;
        if !timeframe.aligned(c.ts) {
            bail!("Candle niet op timeframegrens");
        }
        map.insert(c.ts, c);
    }
    let candles: Vec<_> = map.into_values().collect();
    let gaps = candles
        .windows(2)
        .filter(|w| !timeframe.consecutive(w[0].ts, w[1].ts))
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
        let (cs, g) = normalize(
            &[row("180000"), row("60000"), row("60000")],
            Timeframe::Fixed(60000),
        )
        .unwrap();
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
    fn stamp(date: &str) -> i64 {
        DateTime::parse_from_rfc3339(date)
            .unwrap()
            .timestamp_millis()
    }
    #[test]
    fn month_calendar_including_leap_year() {
        let monthly = Timeframe::parse("1Mutc").unwrap();
        for (a, b) in [
            ("2024-02-01T00:00:00Z", "2024-03-01T00:00:00Z"),
            ("2025-02-01T00:00:00Z", "2025-03-01T00:00:00Z"),
            ("2025-12-01T00:00:00Z", "2026-01-01T00:00:00Z"),
        ] {
            assert!(monthly.aligned(stamp(a)));
            assert_eq!(monthly.next_open(stamp(a)), Some(stamp(b)));
        }
        assert!(!monthly.aligned(stamp("2026-10-02T00:00:00Z")));
        assert!(!monthly.aligned(stamp("2026-10-01T01:00:00Z")));
    }
    #[test]
    fn quarters_start_in_january_april_july_october() {
        let quarter = Timeframe::parse("3Mutc").unwrap();
        assert!(quarter.aligned(stamp("2026-10-01T00:00:00Z")));
        assert!(!quarter.aligned(stamp("2026-11-01T00:00:00Z")));
        assert_eq!(
            quarter.next_open(stamp("2026-10-01T00:00:00Z")),
            Some(stamp("2027-01-01T00:00:00Z"))
        );
    }
    #[test]
    fn utc_weeks_start_on_monday_not_unix_epoch_thursday() {
        let week = Timeframe::parse("1Wutc").unwrap();
        assert!(week.aligned(stamp("2026-09-28T00:00:00Z")));
        assert!(!week.aligned(stamp("2026-10-01T00:00:00Z")));
        assert_eq!(
            week.next_open(stamp("2026-09-28T00:00:00Z")),
            Some(stamp("2026-10-05T00:00:00Z"))
        );
    }
    #[test]
    fn monthly_normalization_counts_real_gaps_only() {
        let dates = [
            "2024-04-01T00:00:00Z",
            "2024-02-01T00:00:00Z",
            "2024-03-01T00:00:00Z",
            "2024-03-01T00:00:00Z",
        ];
        let rows: Vec<_> = dates.iter().map(|d| row(&stamp(d).to_string())).collect();
        let (cs, gaps) = normalize(&rows, Timeframe::CalendarMonths(1)).unwrap();
        assert_eq!(cs.len(), 3);
        assert_eq!(gaps, 0);
        let (_, gaps) = normalize(
            &[rows[0].clone(), rows[1].clone()],
            Timeframe::CalendarMonths(1),
        )
        .unwrap();
        assert_eq!(gaps, 1);
    }
    #[test]
    fn new_intraday_intervals_and_monthly_fixed_duration_guard() {
        for bar in [
            "3m", "30m", "2H", "6Hutc", "12Hutc", "1Wutc", "1Mutc", "3Mutc",
        ] {
            assert!(Timeframe::parse(bar).is_some());
        }
        assert!(interval("1Mutc").is_none());
        assert!(interval("3Mutc").is_none());
        assert!(Timeframe::parse("13m").is_none());
    }
}
