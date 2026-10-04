//! Incremental display analytics. Never used for monetary sizing or order validation.
//! Warm-up values are null; no backfilled or invented indicator values.
use crate::model::Candle;
use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;
use std::collections::VecDeque;
#[derive(Clone, Default)]
struct Average {
    n: usize,
    count: usize,
    sum: f64,
    value: Option<f64>,
    wilder: bool,
}
impl Average {
    fn new(n: usize, wilder: bool) -> Self {
        Self {
            n,
            wilder,
            ..Self::default()
        }
    }
    fn next(&mut self, x: f64) -> Option<f64> {
        self.count += 1;
        match self.value {
            Some(v) => {
                let a = if self.wilder {
                    1.0 / self.n as f64
                } else {
                    2.0 / (self.n + 1) as f64
                };
                self.value = Some(v + a * (x - v));
            }
            None => {
                self.sum += x;
                if self.count == self.n {
                    self.value = Some(self.sum / self.n as f64);
                }
            }
        }
        self.value
    }
}
#[derive(Clone, Default)]
struct Window {
    n: usize,
    q: VecDeque<f64>,
    sum: f64,
    square: f64,
}
impl Window {
    fn new(n: usize) -> Self {
        Self {
            n,
            ..Self::default()
        }
    }
    fn push(&mut self, x: f64) -> Option<f64> {
        self.q.push_back(x);
        self.sum += x;
        self.square += x * x;
        if self.q.len() > self.n {
            let v = self.q.pop_front().unwrap();
            self.sum -= v;
            self.square -= v * v;
        }
        (self.q.len() == self.n).then(|| self.sum / self.n as f64)
    }
}
#[derive(Clone, Serialize, Debug)]
pub struct Point {
    pub ts: i64,
    pub sma: Option<f64>,
    pub ema: Option<f64>,
    pub rsi: Option<f64>,
    pub macd: Option<f64>,
    pub signal: Option<f64>,
    pub atr: Option<f64>,
    pub bb_upper: Option<f64>,
    pub bb_lower: Option<f64>,
    pub vwap: Option<f64>,
    pub stochastic: Option<f64>,
    pub adx: Option<f64>,
    pub obv: f64,
}
#[derive(Clone)]
pub struct Engine {
    sma: Window,
    stochastic_period: usize,
    bb_multiplier: f64,
    ema: Average,
    fast: Average,
    slow: Average,
    signal: Average,
    gain: Average,
    loss: Average,
    atr: Average,
    plus: Average,
    minus: Average,
    adx: Average,
    ranges: VecDeque<(f64, f64)>,
    prev: Option<(f64, f64, f64)>,
    obv: f64,
    pv: f64,
    volume: f64,
    day: i64,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            sma: Window::new(20),
            stochastic_period: 14,
            bb_multiplier: 2.0,
            ema: Average::new(20, false),
            fast: Average::new(12, false),
            slow: Average::new(26, false),
            signal: Average::new(9, false),
            gain: Average::new(14, true),
            loss: Average::new(14, true),
            atr: Average::new(14, true),
            plus: Average::new(14, true),
            minus: Average::new(14, true),
            adx: Average::new(14, true),
            ranges: VecDeque::new(),
            prev: None,
            obv: 0.0,
            pv: 0.0,
            volume: 0.0,
            day: -1,
        }
    }
}
impl Engine {
    pub fn push(&mut self, c: &Candle) -> Point {
        let close = c.close.to_f64().unwrap();
        let high = c.high.to_f64().unwrap();
        let low = c.low.to_f64().unwrap();
        let vol = c.volume.to_f64().unwrap();
        let sma = self.sma.push(close);
        let ema = self.ema.next(close);
        let macd = self
            .fast
            .next(close)
            .zip(self.slow.next(close))
            .map(|(f, s)| f - s);
        let signal = macd.and_then(|m| self.signal.next(m));
        let mut rsi = None;
        let mut atr = None;
        let mut adx = None;
        if let Some((ph, pl, pc)) = self.prev {
            let delta = close - pc;
            let g = self.gain.next(delta.max(0.0));
            let l = self.loss.next((-delta).max(0.0));
            rsi = g.zip(l).map(|(g, l)| {
                if g + l == 0.0 {
                    50.0
                } else {
                    100.0 * g / (g + l)
                }
            });
            atr = self
                .atr
                .next((high - low).max((high - pc).abs()).max((low - pc).abs()));
            let up = high - ph;
            let down = pl - low;
            let plus = self.plus.next(if up > down && up > 0.0 { up } else { 0.0 });
            let minus = self
                .minus
                .next(if down > up && down > 0.0 { down } else { 0.0 });
            if let (Some(p), Some(m), Some(a)) = (plus, minus, atr) {
                let dx = if a == 0.0 || p + m == 0.0 {
                    0.0
                } else {
                    100.0 * (p - m).abs() / (p + m)
                };
                adx = self.adx.next(dx);
            }
            if delta > 0.0 {
                self.obv += vol;
            } else if delta < 0.0 {
                self.obv -= vol;
            }
        }
        self.prev = Some((high, low, close));
        self.ranges.push_back((high, low));
        if self.ranges.len() > self.stochastic_period {
            self.ranges.pop_front();
        }
        let stochastic = if self.ranges.len() == self.stochastic_period {
            let h = self
                .ranges
                .iter()
                .map(|x| x.0)
                .fold(f64::NEG_INFINITY, f64::max);
            let l = self
                .ranges
                .iter()
                .map(|x| x.1)
                .fold(f64::INFINITY, f64::min);
            Some(if h == l {
                50.0
            } else {
                100.0 * (close - l) / (h - l)
            })
        } else {
            None
        };
        if c.ts / 86_400_000 != self.day {
            self.day = c.ts / 86_400_000;
            self.pv = 0.0;
            self.volume = 0.0;
        }
        self.pv += (high + low + close) / 3.0 * vol;
        self.volume += vol;
        let vwap = (self.volume > 0.0).then(|| self.pv / self.volume);
        let sd = sma.map(|m| {
            (self.sma.square / self.sma.n as f64 - m * m)
                .max(0.0)
                .sqrt()
        });
        Point {
            ts: c.ts,
            sma,
            ema,
            rsi,
            macd,
            signal,
            atr,
            bb_upper: sma.zip(sd).map(|(m, s)| m + self.bb_multiplier * s),
            bb_lower: sma.zip(sd).map(|(m, s)| m - self.bb_multiplier * s),
            vwap,
            stochastic,
            adx,
            obv: self.obv,
        }
    }
}
#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub ma: usize,
    pub oscillator: usize,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    pub bb_multiplier: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            ma: 20,
            oscillator: 14,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            bb_multiplier: 2.0,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            [
                self.ma,
                self.oscillator,
                self.macd_fast,
                self.macd_slow,
                self.macd_signal
            ]
            .iter()
            .all(|n| (2..=100).contains(n))
                && self.macd_fast < self.macd_slow
                && self.bb_multiplier.is_finite()
                && (0.5..=5.0).contains(&self.bb_multiplier),
            "Indicatorinputs buiten bereik"
        );
        Ok(())
    }
    fn engine(&self) -> Engine {
        Engine {
            sma: Window::new(self.ma),
            ema: Average::new(self.ma, false),
            fast: Average::new(self.macd_fast, false),
            slow: Average::new(self.macd_slow, false),
            signal: Average::new(self.macd_signal, false),
            gain: Average::new(self.oscillator, true),
            loss: Average::new(self.oscillator, true),
            atr: Average::new(self.oscillator, true),
            plus: Average::new(self.oscillator, true),
            minus: Average::new(self.oscillator, true),
            adx: Average::new(self.oscillator, true),
            stochastic_period: self.oscillator,
            bb_multiplier: self.bb_multiplier,
            ..Engine::default()
        }
    }
}
pub fn calculate(candles: &[Candle], step: i64) -> Vec<Point> {
    calculate_with(candles, step, &Settings::default())
}
pub fn calculate_with(candles: &[Candle], step: i64, settings: &Settings) -> Vec<Point> {
    let mut engine = settings.engine();
    let mut prev = None;
    candles
        .iter()
        .map(|c| {
            if prev.is_some_and(|ts| c.ts - ts != step) {
                engine = settings.engine();
            }
            prev = Some(c.ts);
            if c.closed {
                engine.push(c)
            } else {
                engine.clone().push(c)
            }
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    fn candle(i: i64, price: i64) -> Candle {
        Candle {
            ts: i * 60000,
            open: Decimal::from(price),
            high: Decimal::from(price + 1),
            low: Decimal::from(price - 1),
            close: Decimal::from(price),
            volume: Decimal::from(10),
            closed: true,
        }
    }
    #[test]
    fn warmup_is_missing() {
        let p = Engine::default().push(&candle(1, 100));
        assert!(p.sma.is_none() && p.rsi.is_none() && p.atr.is_none());
    }
    #[test]
    fn rising_reference() {
        let cs: Vec<_> = (1..61).map(|i| candle(i, 100 + i)).collect();
        let p = calculate(&cs, 60000).pop().unwrap();
        assert_eq!(p.rsi, Some(100.0));
        assert!((p.sma.unwrap() - 150.5).abs() < 1e-9);
        assert!((p.ema.unwrap() - 150.5).abs() < 1e-9);
        assert!((p.macd.unwrap() - 7.0).abs() < 1e-9);
        assert!((p.signal.unwrap() - 7.0).abs() < 1e-9);
        assert_eq!(p.atr, Some(2.0));
        assert_eq!(p.adx, Some(100.0));
        assert_eq!(p.obv, 590.0);
        assert!((p.stochastic.unwrap() - 100.0 * 14.0 / 15.0).abs() < 1e-9);
        assert!((p.bb_upper.unwrap() - (150.5 + 2.0 * (33.25_f64).sqrt())).abs() < 1e-8);
        assert!((p.vwap.unwrap() - 130.5).abs() < 1e-9);
    }
    #[test]
    fn flat_rsi_defined() {
        let cs: Vec<_> = (1..50).map(|i| candle(i, 100)).collect();
        assert_eq!(calculate(&cs, 60000).last().unwrap().rsi, Some(50.0));
    }
    #[test]
    fn gaps_reset_warmup() {
        let mut cs: Vec<_> = (1..25).map(|i| candle(i, 100)).collect();
        cs.push(candle(100, 100));
        assert!(calculate(&cs, 60000).last().unwrap().sma.is_none());
    }
    #[test]
    fn open_candle_does_not_commit() {
        let mut e = Engine::default();
        let c = candle(1, 100);
        let _ = e.clone().push(&c);
        assert_eq!(e.push(&c).obv, 0.0);
        assert_eq!(e.sma.q.len(), 1);
    }
    #[test]
    fn zero_volume_has_no_vwap() {
        let mut c = candle(1, 100);
        c.volume = Decimal::ZERO;
        assert!(Engine::default().push(&c).vwap.is_none());
    }
}
