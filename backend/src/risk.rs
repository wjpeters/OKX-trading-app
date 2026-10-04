//! Deterministic PAPER_LOCAL risk authority and durable aggregate state.
//! No HTTP, exchange credentials, floating-point money or model calls belong here.
use anyhow::{Result, bail, ensure};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub fn d(s: &str) -> D {
    s.parse().expect("decimal literal")
}
fn zero() -> D {
    D::ZERO
}
fn min(a: D, b: D) -> D {
    a.min(b)
}
fn floor(value: D, step: D) -> D {
    (value / step).floor() * step
}
fn ceil(value: D, step: D) -> D {
    (value / step).ceil() * step
}
const DAY: i64 = 86_400_000;
pub const FEE_BPS: i64 = 10; // Explicit conservative paper assumption, NOT account fees.
pub const LATENCY_MS: i64 = 500;
pub const MAX_RECORDS: usize = 5000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub capital: D,
    pub trade_risk: D,
    pub daily_loss: D,
    pub total_loss: D,
    pub drawdown: D,
    pub position: D,
    pub positions: usize,
    pub orders: usize,
    pub leverage: D,
    pub spread_bps: D,
    pub slippage_bps: D,
    pub data_age_ms: i64,
    pub orders_per_minute: usize,
    pub loss_streak: u32,
    pub ai_budget: D,
}
impl Limits {
    pub fn preset(capital: D) -> Self {
        Self {
            capital,
            trade_risk: capital * d("0.01"),
            daily_loss: capital * d("0.03"),
            total_loss: capital * d("0.1"),
            drawdown: capital * d("0.08"),
            position: capital * d("0.25"),
            positions: 4,
            orders: 8,
            leverage: D::ONE,
            spread_bps: d("30"),
            slippage_bps: d("20"),
            data_age_ms: 5000,
            orders_per_minute: 6,
            loss_streak: 3,
            ai_budget: D::ZERO,
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.capital >= d("10") && self.capital <= d("1000000"),
            "Kapitaal moet €10–1.000.000 zijn"
        );
        for v in [
            self.trade_risk,
            self.daily_loss,
            self.total_loss,
            self.drawdown,
            self.position,
        ] {
            ensure!(
                v > D::ZERO && v <= self.capital,
                "Risicolimieten moeten positief en maximaal het toegewezen kapitaal zijn"
            );
        }
        ensure!(
            (1..=20).contains(&self.positions) && (1..=40).contains(&self.orders),
            "Positie-/orderlimiet ongeldig"
        );
        ensure!(
            self.leverage == D::ONE && self.ai_budget >= D::ZERO && self.ai_budget <= d("100"),
            "Alleen spot 1×; afzonderlijk AI-budget 0–100 USD"
        );
        ensure!(
            self.spread_bps > D::ZERO
                && self.spread_bps <= d("200")
                && self.slippage_bps > D::ZERO
                && self.slippage_bps <= d("100"),
            "Spread/slippage buiten bereik"
        );
        ensure!(
            (500..=15000).contains(&self.data_age_ms)
                && (1..=60).contains(&self.orders_per_minute)
                && (1..=20).contains(&self.loss_streak),
            "Ouderdom/frequentie/verliesreeks ongeldig"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ledger {
    pub cash: D,
    pub equity: D,
    pub baseline: D,
    pub day_start: D,
    pub high_water: D,
    pub day: i64,
    pub daily_loss: D,
    pub total_loss: D,
    pub drawdown: D,
    pub fees: D,
    pub realized: D,
    pub loss_streak: u32,
    pub paused: bool,
    pub reason: String,
}
impl Ledger {
    fn new(capital: D, now: i64) -> Self {
        Self {
            cash: capital,
            equity: capital,
            baseline: capital,
            day_start: capital,
            high_water: capital,
            day: now / DAY,
            daily_loss: zero(),
            total_loss: zero(),
            drawdown: zero(),
            fees: zero(),
            realized: zero(),
            loss_streak: 0,
            paused: false,
            reason: String::new(),
        }
    }
    fn mark(&mut self, equity: D, now: i64, limits: &Limits) {
        if now / DAY > self.day {
            self.day_start = self.equity;
            self.day = now / DAY;
        }
        self.equity = equity;
        self.high_water = self.high_water.max(equity);
        self.daily_loss = (self.day_start - equity).max(zero());
        self.total_loss = (self.baseline - equity).max(zero());
        self.drawdown = (self.high_water - equity).max(zero());
        if let Some(reason) = self.breach(limits) {
            self.paused = true;
            self.reason = reason.into();
        }
    }
    fn breach(&self, l: &Limits) -> Option<&'static str> {
        if self.daily_loss >= l.daily_loss {
            Some("DAY_LOSS")
        } else if self.total_loss >= l.total_loss {
            Some("TOTAL_LOSS")
        } else if self.drawdown >= l.drawdown {
            Some("DRAWDOWN")
        } else if self.loss_streak >= l.loss_streak {
            Some("LOSS_STREAK")
        } else if self.equity <= zero() {
            Some("NO_EQUITY")
        } else {
            None
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bot {
    pub id: String,
    pub name: String,
    pub instrument: String,
    pub version: String,
    pub limits: Limits,
    pub ledger: Ledger,
    pub running: bool,
    pub last_bar: i64,
    pub last_signal: String,
    pub fast: usize,
    pub slow: usize,
    pub order_eur: D,
    pub stop_bps: D,
    pub target_bps: D,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderState {
    Intent,
    PendingSubmit,
    Accepted,
    PartialFill,
    Filled,
    PendingCancel,
    Canceled,
    Rejected,
    Unknown,
}
impl OrderState {
    pub fn open(&self) -> bool {
        matches!(
            self,
            Self::Intent
                | Self::PendingSubmit
                | Self::Accepted
                | Self::PartialFill
                | Self::PendingCancel
                | Self::Unknown
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Buy,
    Sell,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Kind {
    Market,
    Limit,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub client_id: String,
    pub bot_id: String,
    pub instrument: String,
    pub side: Side,
    pub kind: Kind,
    pub quantity: D,
    pub limit: Option<D>,
    pub stop: Option<D>,
    pub target: Option<D>,
    pub position_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub intent: Intent,
    pub state: OrderState,
    pub quantity: D,
    pub filled: D,
    pub reserve: D,
    pub cap: D,
    pub created_ms: i64,
    pub updated_ms: i64,
    pub eligible_ms: i64,
    pub expires_ms: i64,
    pub reason: String,
    pub book_ts: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub bot_id: String,
    pub instrument: String,
    pub quantity: D,
    pub cost: D,
    pub stop: D,
    pub target: Option<D>,
    pub realized: D,
    pub closing_order: Option<String>,
    pub protection_state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fill {
    pub id: String,
    pub order_id: String,
    pub instrument: String,
    pub side: Side,
    pub quantity: D,
    pub price: D,
    pub fee: D,
    pub ts: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    pub ts: i64,
    pub kind: String,
    pub subject: String,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Paper {
    pub schema: u32,
    pub mode: String,
    pub currency: String,
    pub timezone: String,
    pub revision: u64,
    pub limits: Limits,
    pub ledger: Ledger,
    pub bots: BTreeMap<String, Bot>,
    pub orders: BTreeMap<String, Order>,
    pub positions: BTreeMap<String, Position>,
    pub fills: Vec<Fill>,
    pub events: Vec<Event>,
    pub consumed: BTreeMap<String, (i64, D, D)>,
    pub valuation_fresh: bool,
    pub heartbeat_ms: i64,
    pub last_mark_ms: i64,
    pub flatten: bool,
}
#[derive(Clone, Debug)]
pub struct Quote {
    pub instrument: String,
    pub base: String,
    pub quote: String,
    pub bid: D,
    pub ask: D,
    pub bid_size: D,
    pub ask_size: D,
    pub tick: D,
    pub lot: D,
    pub minimum: D,
    pub ts: i64,
    pub received_ms: i64,
}
impl Quote {
    pub fn fresh(&self, now: i64, age: i64) -> bool {
        self.quote == "EUR"
            && self.instrument == format!("{}-EUR", self.base)
            && self.bid > zero()
            && self.ask >= self.bid
            && self.ask <= d("100000000")
            && self.tick > zero()
            && self.lot > zero()
            && self.minimum > zero()
            && (0..=age).contains(&(now - self.received_ms))
            && (-1000..=age).contains(&(now - self.ts))
    }
    fn spread(&self) -> D {
        (self.ask - self.bid) / self.bid * d("10000")
    }
}
pub type Quotes = BTreeMap<String, Quote>;
// Construction is private. UI, scripts and proposal envelopes cannot mint this capability.
struct AuthorizedIntent {
    quantity: D,
    cap: D,
    reserve: D,
}

impl Paper {
    pub fn new(limits: Limits, manual: Limits, now: i64) -> Result<Self> {
        limits.validate()?;
        manual.validate()?;
        ensure!(
            manual.ai_budget <= limits.ai_budget,
            "Handmatig AI-deelbudget overschrijdt rekeningbudget"
        );
        ensure!(
            manual.capital <= limits.capital,
            "Handmatig budget groter dan rekening"
        );
        let mut s = Self {
            schema: 2,
            mode: "PAPER_LOCAL".into(),
            currency: "EUR".into(),
            timezone: "UTC".into(),
            revision: 0,
            ledger: Ledger::new(limits.capital, now),
            limits,
            bots: BTreeMap::new(),
            orders: BTreeMap::new(),
            positions: BTreeMap::new(),
            fills: vec![],
            events: vec![],
            consumed: BTreeMap::new(),
            valuation_fresh: true,
            heartbeat_ms: now,
            last_mark_ms: now,
            flatten: false,
        };
        s.bots.insert(
            "manual".into(),
            Bot {
                id: "manual".into(),
                name: "Handmatig".into(),
                instrument: String::new(),
                version: "manual-v1".into(),
                ledger: Ledger::new(manual.capital, now),
                limits: manual,
                running: false,
                last_bar: 0,
                last_signal: "Nog geen opdracht".into(),
                fast: 20,
                slow: 50,
                order_eur: zero(),
                stop_bps: zero(),
                target_bps: zero(),
            },
        );
        s.event(
            now,
            "ACCOUNT_CREATED",
            "paper",
            "Virtueel startkapitaal; UTC daggrens; geen stortingen/opnames, funding of AI-kosten",
        );
        Ok(s)
    }
    pub fn event(&mut self, now: i64, kind: &str, subject: &str, detail: &str) {
        self.revision += 1;
        self.events.push(Event {
            seq: self.revision,
            ts: now,
            kind: kind.into(),
            subject: subject.into(),
            detail: detail.chars().take(500).collect(),
        });
        // Full event history is appended transactionally to PostgreSQL before trimming.
    }
    pub fn reserve(&self, bot: Option<&str>) -> D {
        self.orders
            .values()
            .filter(|o| {
                o.state.open()
                    && o.intent.side == Side::Buy
                    && bot.is_none_or(|b| o.intent.bot_id == b)
            })
            .map(|o| o.reserve)
            .sum()
    }
    pub fn mark(&mut self, quotes: &Quotes, now: i64) {
        let fresh = self
            .positions
            .values()
            .filter(|p| p.quantity > zero())
            .all(|p| {
                let age = self
                    .limits
                    .data_age_ms
                    .min(self.bots[&p.bot_id].limits.data_age_ms);
                quotes.get(&p.instrument).is_some_and(|q| q.fresh(now, age))
            });
        self.valuation_fresh = fresh;
        if !fresh {
            return;
        }
        let exit_fee = D::ONE - d(&FEE_BPS.to_string()) / d("10000");
        let mut values: BTreeMap<String, D> = BTreeMap::new();
        for p in self.positions.values().filter(|p| p.quantity > zero()) {
            *values.entry(p.bot_id.clone()).or_default() +=
                p.quantity * quotes[&p.instrument].bid * exit_fee;
        }
        let previous = self.ledger.paused;
        self.ledger.mark(
            self.ledger.cash + values.values().copied().sum::<D>(),
            now,
            &self.limits,
        );
        if !previous && self.ledger.paused {
            let reason = self.ledger.reason.clone();
            self.event(now, "RISK_STOP", "account", &reason);
        }
        let mut stops = vec![];
        for b in self.bots.values_mut() {
            let prev = b.ledger.paused;
            b.ledger.mark(
                b.ledger.cash + values.get(&b.id).copied().unwrap_or_default(),
                now,
                &b.limits,
            );
            if !prev && b.ledger.paused {
                stops.push((b.id.clone(), b.ledger.reason.clone()));
            }
        }
        for (id, reason) in stops {
            self.event(now, "RISK_STOP", &id, &reason);
        }
        self.last_mark_ms = now;
        // Loss stops cancel entry orders; manual PAUSE alone preserves them.
        let ids: Vec<_> = self
            .orders
            .values()
            .filter(|o| {
                o.state.open()
                    && o.intent.side == Side::Buy
                    && ((self.ledger.paused && self.ledger.reason != "USER_PAUSE")
                        || (self.bots[&o.intent.bot_id].ledger.paused
                            && self.bots[&o.intent.bot_id].ledger.reason != "USER_PAUSE"))
            })
            .map(|o| o.id.clone())
            .collect();
        for id in ids {
            self.request_cancel(&id, now).ok();
        }
    }
    fn authorize(
        &self,
        i: &Intent,
        q: &Quote,
        now: i64,
        existing: Option<&str>,
    ) -> Result<AuthorizedIntent> {
        ensure!(
            self.mode == "PAPER_LOCAL" || self.mode == "REPLAY",
            "Execution mode verboden"
        );
        ensure!(i.instrument == q.instrument, "Instrument mismatch");
        let b = self
            .bots
            .get(&i.bot_id)
            .ok_or_else(|| anyhow::anyhow!("Onbekende bot"))?;
        ensure!(
            q.fresh(now, self.limits.data_age_ms.min(b.limits.data_age_ms)),
            "STALE_MARKET"
        );
        ensure!(
            i.quantity > zero() && i.quantity <= d("1000000000"),
            "Ongeldige hoeveelheid"
        );
        ensure!(
            floor(i.quantity, q.lot) == i.quantity && i.quantity >= q.minimum,
            "Hoeveelheid voldoet niet aan lot/minimum"
        );

        if let Some(px) = i.limit {
            ensure!(
                px > zero() && px <= d("100000000") && floor(px, q.tick) == px,
                "Ongeldige limietprijs/tick"
            );
        }
        ensure!(
            (i.kind == Kind::Limit) == i.limit.is_some(),
            "Limietprijs vereist uitsluitend voor LIMIT"
        );
        let slip = self.limits.slippage_bps.min(b.limits.slippage_bps) / d("10000");
        let reference = if i.side == Side::Buy { q.ask } else { q.bid };
        let cap = if i.side == Side::Buy {
            floor(reference * (D::ONE + slip), q.tick).min(i.limit.unwrap_or(d("100000000")))
        } else {
            ceil(reference * (D::ONE - slip), q.tick).max(i.limit.unwrap_or(zero()))
        };
        if i.side == Side::Sell {
            ensure!(
                i.stop.is_none() && i.target.is_none(),
                "Een afbouworder mag bescherming niet wijzigen"
            );
            let p = i
                .position_id
                .as_ref()
                .and_then(|id| self.positions.get(id))
                .ok_or_else(|| anyhow::anyhow!("Positie vereist voor spot-afbouw"))?;
            ensure!(
                p.bot_id == i.bot_id && p.instrument == i.instrument,
                "Positie behoort niet bij deze bot/markt"
            );
            let reserved: D = self
                .orders
                .values()
                .filter(|o| {
                    o.state.open()
                        && o.intent.side == Side::Sell
                        && o.intent.position_id == i.position_id
                        && existing != Some(o.id.as_str())
                })
                .map(|o| o.quantity - o.filled)
                .sum();
            ensure!(
                i.quantity <= p.quantity - reserved,
                "Inventaris al gereserveerd of ontoereikend"
            );
            // Stops and reductions bypass entry/loss/frequency/spread limits, never inventory/freshness/slippage.
            return Ok(AuthorizedIntent {
                quantity: i.quantity,
                cap,
                reserve: zero(),
            });
        }
        ensure!(
            self.orders.len() < MAX_RECORDS && self.fills.len() < 20000 || existing.is_some(),
            "ORDER_ARCHIVE_LIMIT: entries geblokkeerd; afbouw blijft mogelijk"
        );
        ensure!(i.position_id.is_none(), "Koop mag geen positie-id opgeven");
        ensure!(self.valuation_fresh, "VALUATION_STALE");
        ensure!(
            !self.ledger.paused,
            "Account PAUSE_ENTRIES: {}",
            self.ledger.reason
        );
        ensure!(!b.ledger.paused, "Bot PAUSE_ENTRIES: {}", b.ledger.reason);
        ensure!(
            self.ledger.breach(&self.limits).is_none() && b.ledger.breach(&b.limits).is_none(),
            "Verlieslimiet bereikt"
        );
        ensure!(b.id == "manual" || b.running, "Bot is gepauzeerd");
        ensure!(
            b.id == "manual" || b.instrument == i.instrument,
            "Bot-instrument vastgezet"
        );
        ensure!(
            q.spread() <= self.limits.spread_bps.min(b.limits.spread_bps),
            "SPREAD_LIMIT"
        );
        let stop = i
            .stop
            .ok_or_else(|| anyhow::anyhow!("Stop-loss verplicht"))?;
        let entry = if i.kind == Kind::Limit {
            i.limit.unwrap().min(cap)
        } else {
            cap
        };
        ensure!(
            stop > zero() && stop < entry && stop < q.bid && floor(stop, q.tick) == stop,
            "Stop moet onder instap/biedprijs liggen en aan tick voldoen"
        );
        if let Some(t) = i.target {
            ensure!(
                t > entry && t <= d("100000000") && floor(t, q.tick) == t,
                "Target moet boven instap liggen en aan tick voldoen"
            );
        }
        let reserve = i.quantity * entry * (D::ONE + d(&FEE_BPS.to_string()) / d("10000"));
        let risk = i.quantity * (entry - stop * (D::ONE - slip))
            + i.quantity * (entry + stop) * d(&FEE_BPS.to_string()) / d("10000");
        ensure!(
            risk <= self.limits.trade_risk.min(b.limits.trade_risk),
            "TRADE_RISK_LIMIT inclusief fees/slippage"
        );
        for (bot, l, cash) in [
            (None, &self.limits, self.ledger.cash),
            (Some(i.bot_id.as_str()), &b.limits, b.ledger.cash),
        ] {
            let orders: Vec<_> = self
                .orders
                .values()
                .filter(|o| {
                    o.state.open()
                        && existing != Some(o.id.as_str())
                        && bot.is_none_or(|id| o.intent.bot_id == id)
                })
                .collect();
            let reserved: D = orders
                .iter()
                .filter(|o| o.intent.side == Side::Buy)
                .map(|o| o.reserve)
                .sum();
            ensure!(
                reserve <= cash - reserved,
                "INSUFFICIENT_UNRESERVED_CAPITAL"
            );
            ensure!(orders.len() < l.orders, "OPEN_ORDER_LIMIT");
            let positions: Vec<_> = self
                .positions
                .values()
                .filter(|p| p.quantity > zero() && bot.is_none_or(|id| p.bot_id == id))
                .collect();
            let deployed: D = positions.iter().map(|p| p.cost).sum();
            ensure!(
                deployed + reserved + reserve <= l.capital,
                "ALLOCATED_CAPITAL_LIMIT"
            );
            let mut symbols: BTreeSet<_> =
                positions.iter().map(|p| p.instrument.as_str()).collect();
            symbols.extend(
                orders
                    .iter()
                    .filter(|o| o.intent.side == Side::Buy)
                    .map(|o| o.intent.instrument.as_str()),
            );
            symbols.insert(&i.instrument);
            ensure!(symbols.len() <= l.positions, "OPEN_POSITION_LIMIT");
            let exposure: D = positions
                .iter()
                .filter(|p| p.instrument == i.instrument)
                .map(|p| (p.quantity * q.ask).max(p.cost))
                .sum();
            let pending: D = orders
                .iter()
                .filter(|o| o.intent.side == Side::Buy && o.intent.instrument == i.instrument)
                .map(|o| o.reserve)
                .sum();
            ensure!(
                exposure + pending + reserve <= l.position,
                "UNDERLYING_EXPOSURE_LIMIT"
            );
            let count = self
                .orders
                .values()
                .filter(|o| {
                    o.intent.side == Side::Buy
                        && o.created_ms > now - 60_000
                        && existing != Some(o.id.as_str())
                        && bot.is_none_or(|id| o.intent.bot_id == id)
                })
                .count();
            ensure!(count < l.orders_per_minute, "ORDER_FREQUENCY_LIMIT");
        }
        Ok(AuthorizedIntent {
            quantity: i.quantity,
            cap: entry,
            reserve,
        })
    }
    pub fn preview(&self, i: &Intent, q: &Quote, now: i64) -> Result<serde_json::Value> {
        let a = self.authorize(i, q, now, None)?;
        Ok(
            serde_json::json!({"quantity":a.quantity,"price_bound":a.cap,"reservation_eur":a.reserve,"fee_bps":FEE_BPS,"latency_ms":LATENCY_MS,"mode":self.mode,"stop":i.stop,"target":i.target}),
        )
    }
    pub fn submit(&mut self, i: Intent, q: &Quote, now: i64) -> Result<String> {
        ensure!(
            !i.client_id.is_empty()
                && i.client_id.len() <= 80
                && i.client_id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c)),
            "Ongeldig client-id"
        );
        if let Some(old) = self
            .orders
            .values()
            .find(|o| o.intent.client_id == i.client_id)
        {
            ensure!(old.intent == i, "Client-id hergebruikt met andere inhoud");
            return Ok(old.id.clone());
        }
        let a = self.authorize(&i, q, now, None)?;
        let id = format!("p-{}", i.client_id);
        self.event(
            now,
            "INTENT",
            &id,
            "Risicokern akkoord; kapitaal/inventaris atomair gereserveerd",
        );
        self.orders.insert(
            id.clone(),
            Order {
                id: id.clone(),
                intent: i,
                state: OrderState::PendingSubmit,
                quantity: a.quantity,
                filled: zero(),
                reserve: a.reserve,
                cap: a.cap,
                created_ms: now,
                updated_ms: now,
                eligible_ms: now + LATENCY_MS,
                expires_ms: now + 86_400_000,
                reason: String::new(),
                book_ts: q.ts,
            },
        );
        if let Some(pid) = self.orders[&id].intent.position_id.clone() {
            let position = self.positions.get_mut(&pid).expect("authorized position");
            position.closing_order = Some(id.clone());
            position.protection_state = "EXIT_PENDING".into();
        }
        self.event(
            now,
            "PENDING_SUBMIT",
            &id,
            "Duurzaam vastleggen vóór adapter; 500 ms papermodelvertraging",
        );
        Ok(id)
    }
    pub fn request_cancel(&mut self, id: &str, now: i64) -> Result<()> {
        let o = self
            .orders
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Order ontbreekt"))?;
        ensure!(
            o.state != OrderState::Unknown,
            "UNKNOWN vereist reconciliatie"
        );
        if !o.state.open() || o.state == OrderState::PendingCancel {
            return Ok(());
        }
        // Protective exits can only be replaced after reconciliation, not casually canceled.
        ensure!(
            !o.reason.starts_with("PROTECT"),
            "Beschermende afbouw blijft actief tot sluiting"
        );
        o.state = OrderState::PendingCancel;
        o.updated_ms = now;
        self.event(
            now,
            "PENDING_CANCEL",
            id,
            "Reservering blijft staan tot adapterbevestiging",
        );
        Ok(())
    }
    fn canceled(&mut self, id: &str, now: i64, reason: &str) {
        let o = self.orders.get_mut(id).unwrap();
        o.state = OrderState::Canceled;
        o.reserve = zero();
        o.updated_ms = now;
        o.reason = reason.into();
        for p in self.positions.values_mut() {
            if p.closing_order.as_deref() == Some(id) {
                p.closing_order = None;
                p.protection_state = if p.quantity > zero() {
                    "ARMED"
                } else {
                    "CLOSED"
                }
                .into();
            }
        }
        self.event(now, "CANCELED", id, reason);
    }
    fn reject(&mut self, id: &str, now: i64, reason: &str) {
        let o = self.orders.get_mut(id).unwrap();
        o.state = OrderState::Rejected;
        o.reserve = zero();
        o.updated_ms = now;
        o.reason = reason.into();
        for p in self.positions.values_mut() {
            if p.closing_order.as_deref() == Some(id) {
                p.closing_order = None;
                p.protection_state = "ARMED".into();
            }
        }
        self.event(now, "REJECTED", id, reason);
    }
    pub fn control(&mut self, action: &str, quotes: &Quotes, now: i64) -> Result<()> {
        match action {
            "PAUSE_ENTRIES" => {
                self.ledger.paused = true;
                self.ledger.reason = "USER_PAUSE".into();
            }
            "CANCEL_ENTRY_ORDERS" => {
                self.ledger.paused = true;
                self.ledger.reason = "USER_PAUSE".into();
                let ids: Vec<_> = self
                    .orders
                    .values()
                    .filter(|o| o.intent.side == Side::Buy && o.state.open())
                    .map(|o| o.id.clone())
                    .collect();
                for id in ids {
                    self.request_cancel(&id, now)?;
                }
            }
            "RESUME_ENTRIES" => {
                self.mark(quotes, now);
                ensure!(
                    self.valuation_fresh && self.ledger.breach(&self.limits).is_none(),
                    "Risicostop/waardering nog niet opgelost"
                );
                ensure!(
                    !self.orders.values().any(|o| o.state == OrderState::Unknown),
                    "Onbekende orderstatus"
                );
                ensure!(
                    quotes
                        .values()
                        .any(|q| q.fresh(now, self.limits.data_age_ms)),
                    "Geen verse marktdata"
                );
                ensure!(
                    !self.flatten || self.positions.values().all(|p| p.quantity == zero()),
                    "Noodafbouw nog niet voltooid"
                );
                self.flatten = false;
                self.ledger.paused = false;
                self.ledger.reason = String::new();
            }
            "EMERGENCY_FLATTEN" => {
                self.control("CANCEL_ENTRY_ORDERS", quotes, now)?;
                self.ledger.reason = "EMERGENCY_FLATTEN".into();
                self.flatten = true;
                // Latch persists. Freshness/price bound may defer execution; protection remains.
            }
            _ => bail!("Onbekende controle"),
        }
        self.event(
            now,
            action,
            "account",
            "Expliciete gebruikersopdracht; limieten/baselines niet gewijzigd",
        );
        Ok(())
    }
    pub fn add_bot(&mut self, mut bot: Bot, now: i64) -> Result<()> {
        bot.limits.validate()?;
        ensure!(
            self.bots.len() < 10 && !self.bots.contains_key(&bot.id) && bot.id != "manual",
            "Bot-id bestaat of maximum bereikt"
        );
        ensure!(
            !bot.id.is_empty()
                && bot.id.len() <= 32
                && bot
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-'),
            "Ongeldig bot-id"
        );
        ensure!(
            !bot.name.is_empty() && bot.name.len() <= 60,
            "Botnaam ongeldig"
        );
        ensure!(
            bot.version == "sma-cross-v1"
                && bot.fast >= 2
                && bot.fast < bot.slow
                && bot.slow <= 100,
            "Strategieparameters ongeldig"
        );
        ensure!(
            bot.order_eur >= d("5") && bot.order_eur <= bot.limits.position,
            "Orderbedrag ongeldig"
        );
        ensure!(
            bot.stop_bps >= d("10")
                && bot.stop_bps <= d("1000")
                && bot.target_bps >= d("10")
                && bot.target_bps <= d("5000"),
            "Stop/target buiten bereik"
        );
        let allocated_ai: D = self.bots.values().map(|b| b.limits.ai_budget).sum();
        ensure!(
            allocated_ai + bot.limits.ai_budget <= self.limits.ai_budget,
            "AI-deelbudgetten overschrijden rekeningbudget in USD"
        );
        let allocated: D = self.bots.values().map(|b| b.limits.capital).sum();
        ensure!(
            allocated + bot.limits.capital <= self.limits.capital,
            "BOT_ALLOCATION_EXCEEDS_ACCOUNT"
        );
        bot.ledger = Ledger::new(bot.limits.capital, now);
        bot.running = false;
        bot.last_bar = 0;
        bot.last_signal = "Gepauzeerd; start bewust".into();
        let id = bot.id.clone();
        self.bots.insert(id.clone(), bot);
        self.event(
            now,
            "BOT_CREATED",
            &id,
            "Versie/parameters/limieten vastgezet; start gepauzeerd",
        );
        Ok(())
    }
    pub fn bot_control(
        &mut self,
        id: &str,
        running: bool,
        quotes: &Quotes,
        now: i64,
    ) -> Result<()> {
        self.mark(quotes, now);
        let b = self
            .bots
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Bot ontbreekt"))?;
        ensure!(id != "manual", "Handmatig is geen automatische strategie");
        if running {
            ensure!(
                self.valuation_fresh && !self.ledger.paused && b.ledger.breach(&b.limits).is_none(),
                "Risicostop nog actief"
            );
            ensure!(
                quotes.get(&b.instrument).is_some_and(
                    |q| q.fresh(now, b.limits.data_age_ms.min(self.limits.data_age_ms))
                ),
                "Geen verse marktdata"
            );
            b.ledger.paused = false;
            b.ledger.reason = String::new();
        }
        b.running = running;
        if !running {
            let ids: Vec<_> = self
                .orders
                .values()
                .filter(|o| o.intent.bot_id == id && o.intent.side == Side::Buy && o.state.open())
                .map(|o| o.id.clone())
                .collect();
            for oid in ids {
                self.request_cancel(&oid, now)?;
            }
        }
        self.event(
            now,
            if running { "BOT_STARTED" } else { "BOT_PAUSED" },
            id,
            "Bescherming blijft actief",
        );
        Ok(())
    }
    fn protection(&mut self, quotes: &Quotes, now: i64) {
        let positions: Vec<_> = self
            .positions
            .values()
            .filter(|p| p.quantity > zero())
            .cloned()
            .collect();
        for p in positions {
            let Some(q) = quotes.get(&p.instrument) else {
                continue;
            };
            let emergency = self.flatten;
            if !emergency && q.bid > p.stop && p.target.is_none_or(|t| q.bid < t) {
                continue;
            }
            let age = self
                .limits
                .data_age_ms
                .min(self.bots[&p.bot_id].limits.data_age_ms);
            if !q.fresh(now, age) {
                continue;
            }
            // Stop wins over a pending user exit; cancellation is atomic in this local adapter.
            if let Some(id) = p.closing_order.as_ref() {
                if self
                    .orders
                    .get(id)
                    .is_some_and(|o| o.state.open() && o.reason.starts_with("PROTECT"))
                {
                    continue;
                }
                if self
                    .orders
                    .get(id)
                    .is_some_and(|o| o.state == OrderState::Unknown)
                {
                    continue;
                }
                if self.orders.get(id).is_some_and(|o| o.state.open()) {
                    self.canceled(id, now, "Vervangen door beschermende afbouw");
                }
            }
            if self.orders.get(&p.id).is_some_and(|o| o.state.open()) {
                self.canceled(&p.id, now, "Entryrest geannuleerd vóór afbouw");
            }
            let qty = floor(p.quantity, q.lot);
            if qty < q.minimum {
                if self.positions[&p.id].protection_state == "DUST" {
                    continue;
                }
                self.positions.get_mut(&p.id).unwrap().protection_state = "DUST".into();
                self.event(now,"DUST_PROTECTION",&p.id,"Onder minimumorder; resterende positie zichtbaar, niet als gesloten gemarkeerd");
                continue;
            }
            let intent = Intent {
                client_id: format!("protect-{}-{}", p.id, self.revision),
                bot_id: p.bot_id.clone(),
                instrument: p.instrument.clone(),
                side: Side::Sell,
                kind: Kind::Market,
                quantity: qty,
                limit: None,
                stop: None,
                target: None,
                position_id: Some(p.id.clone()),
            };
            if let Ok(id) = self.submit(intent, q, now) {
                self.orders.get_mut(&id).unwrap().reason = "PROTECT: stop/target/emergency".into();
                self.positions.get_mut(&p.id).unwrap().closing_order = Some(id);
                self.positions.get_mut(&p.id).unwrap().protection_state = "EXIT_PENDING".into();
            }
        }
    }
    pub fn tick(&mut self, quotes: &Quotes, now: i64) {
        if now < self.heartbeat_ms {
            self.ledger.paused = true;
            self.ledger.reason = "CLOCK_ROLLBACK".into();
            return;
        }
        self.heartbeat_ms = now;
        self.mark(quotes, now);
        // Each call runs under a PostgreSQL account-row lock. Paper has no external side effect.
        let mut ids: Vec<_> = self.orders.keys().cloned().collect();
        ids.sort_by_key(|id| (self.orders[id].created_ms, id.clone()));
        for id in ids {
            let o = self.orders[&id].clone();
            if o.state == OrderState::PendingCancel {
                self.canceled(&id, now, "Lokale adapter bevestigt annulering");
                continue;
            }
            if !o.state.open() || o.state == OrderState::Unknown || now < o.eligible_ms {
                continue;
            }
            if now > o.expires_ms && o.intent.side == Side::Buy {
                self.canceled(&id, now, "Order vervallen na 24 uur");
                continue;
            }
            let Some(q) = quotes.get(&o.intent.instrument) else {
                continue;
            };
            if !q.fresh(
                now,
                self.limits
                    .data_age_ms
                    .min(self.bots[&o.intent.bot_id].limits.data_age_ms),
            ) {
                continue;
            }
            if o.state == OrderState::PendingSubmit {
                // Second risk check at gateway. Original cap cannot loosen on recheck.
                match self.authorize(&o.intent, q, now, Some(&id)) {
                    Ok(_) => {
                        let x = self.orders.get_mut(&id).unwrap();
                        x.state = OrderState::Accepted;
                        x.updated_ms = now;
                        self.event(
                            now,
                            "ACCEPTED",
                            &id,
                            "Lokale acknowledgement; nog geen fill",
                        );
                    }
                    Err(e) => {
                        self.reject(&id, now, &e.to_string());
                    }
                }
                continue; // acknowledgement and fill are separately durable commits
            }
            if o.intent.side == Side::Buy
                && (self.ledger.paused || self.bots[&o.intent.bot_id].ledger.paused)
            {
                continue;
            }
            // Gate repeats before every partial fill, with only the remaining quantity.
            let mut remaining = o.intent.clone();
            remaining.quantity = o.quantity - o.filled;
            if remaining.quantity < q.minimum {
                self.canceled(&id, now, "Resterende hoeveelheid onder minimum");
                continue;
            }
            if self.authorize(&remaining, q, now, Some(&id)).is_err() {
                continue;
            }
            let px = if o.intent.side == Side::Buy {
                q.ask
            } else {
                q.bid
            };
            if (o.intent.side == Side::Buy && px > o.cap)
                || (o.intent.side == Side::Sell && px < o.cap)
            {
                continue;
            }
            if o.intent.kind == Kind::Limit
                && ((o.intent.side == Side::Buy && px >= o.intent.limit.unwrap())
                    || (o.intent.side == Side::Sell && px <= o.intent.limit.unwrap()))
            {
                continue;
            } // touch is not a fill
            let used = self
                .consumed
                .entry(q.instrument.clone())
                .or_insert((q.ts, zero(), zero()));
            if q.ts < used.0 {
                continue;
            }
            if q.ts > used.0 {
                *used = (q.ts, zero(), zero());
            }
            // Conservative, explicitly estimated participation: 10% of best displayed level.
            let available = if o.intent.side == Side::Buy {
                q.ask_size * d("0.1") - used.2
            } else {
                q.bid_size * d("0.1") - used.1
            };
            let qty = floor(min(o.quantity - o.filled, available.max(zero())), q.lot);
            if qty < q.minimum {
                continue;
            }
            let fill_id = format!("{}-{}-{}", id, q.ts, o.filled);
            if let Err(e) = self.apply_fill(&id, &fill_id, qty, px, now) {
                let x = self.orders.get_mut(&id).unwrap();
                x.state = OrderState::Unknown;
                x.reason = e.to_string();
                self.ledger.paused = true;
                self.ledger.reason = "PAPER_FILL_INVARIANT".into();
                self.event(now, "UNKNOWN", &id, &e.to_string());
                continue;
            }
            let used = self.consumed.get_mut(&q.instrument).unwrap();
            if o.intent.side == Side::Buy {
                used.2 += qty;
            } else {
                used.1 += qty;
            }
        }
        self.mark(quotes, now);
        self.protection(quotes, now);
    }
    fn apply_fill(&mut self, id: &str, fill_id: &str, qty: D, price: D, now: i64) -> Result<()> {
        if self.fills.iter().any(|f| f.id == fill_id) {
            return Ok(());
        }
        let o = self
            .orders
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Onbekende order"))?
            .clone();
        ensure!(
            matches!(o.state, OrderState::Accepted | OrderState::PartialFill),
            "Fill vóór acceptatie of na eindstatus"
        );
        ensure!(
            qty > zero() && qty <= o.quantity - o.filled && price > zero(),
            "Overfill/ongeldige fill"
        );
        ensure!(
            (o.intent.side == Side::Buy && price <= o.cap)
                || (o.intent.side == Side::Sell && price >= o.cap),
            "Prijsgrens overschreden"
        );
        let fee = qty * price * d(&FEE_BPS.to_string()) / d("10000");
        let cash = qty * price;
        let b = self.bots.get_mut(&o.intent.bot_id).unwrap();
        if o.intent.side == Side::Buy {
            ensure!(
                cash + fee <= o.reserve
                    && cash + fee <= self.ledger.cash
                    && cash + fee <= b.ledger.cash,
                "Fill groter dan reserve"
            );
            self.ledger.cash -= cash + fee;
            b.ledger.cash -= cash + fee;
            let p = self.positions.entry(id.into()).or_insert(Position {
                id: id.into(),
                bot_id: b.id.clone(),
                instrument: o.intent.instrument.clone(),
                quantity: zero(),
                cost: zero(),
                stop: o.intent.stop.unwrap(),
                target: o.intent.target,
                realized: zero(),
                closing_order: None,
                protection_state: "ARMED".into(),
            });
            p.quantity += qty;
            p.cost += cash + fee;
        } else {
            let p = self
                .positions
                .get_mut(o.intent.position_id.as_ref().unwrap())
                .ok_or_else(|| anyhow::anyhow!("Positie ontbreekt"))?;
            ensure!(qty <= p.quantity, "Geen short selling");
            let cost = p.cost * qty / p.quantity;
            let pnl = cash - fee - cost;
            p.quantity -= qty;
            p.cost -= cost;
            p.realized += pnl;
            self.ledger.cash += cash - fee;
            b.ledger.cash += cash - fee;
            self.ledger.realized += pnl;
            b.ledger.realized += pnl;
            if p.quantity == zero() {
                if p.realized < zero() {
                    self.ledger.loss_streak += 1;
                    b.ledger.loss_streak += 1;
                } else {
                    self.ledger.loss_streak = 0;
                    b.ledger.loss_streak = 0;
                }
                p.closing_order = None;
            }
        }
        self.ledger.fees += fee;
        b.ledger.fees += fee;
        let x = self.orders.get_mut(id).unwrap();
        x.filled += qty;
        x.updated_ms = now;
        x.state = if x.filled == x.quantity {
            OrderState::Filled
        } else {
            OrderState::PartialFill
        };
        x.reserve = if o.intent.side == Side::Buy {
            (x.quantity - x.filled) * x.cap * (D::ONE + d("0.001"))
        } else {
            zero()
        };
        if x.state == OrderState::Filled {
            if let Some(pid) = &x.intent.position_id {
                if let Some(p) = self.positions.get_mut(pid) {
                    p.closing_order = None;
                }
            }
        }
        self.fills.push(Fill {
            id: fill_id.into(),
            order_id: id.into(),
            instrument: o.intent.instrument,
            side: o.intent.side,
            quantity: qty,
            price,
            fee,
            ts: now,
        });
        let state = if x.state == OrderState::Filled {
            "FILLED"
        } else {
            "PARTIAL_FILL"
        };
        self.event(
            now,
            "FILL",
            id,
            &format!("{qty} @ {price}; fee {fee}; {state}"),
        );
        Ok(())
    }
    pub fn recover(&mut self, quotes: &Quotes, now: i64) {
        // Local fills/state/reserves share one transaction. No external accepted order can exist.
        // UNKNOWN is deliberately held for review, never converted into a new submission.
        self.mark(quotes, now);
        self.event(
            now,
            "RECOVERED",
            "paper",
            "Duurzame lokale state geladen; pending intents behouden; geen exchange-retry",
        );
    }
}

pub fn sma_signal(closes: &[D], fast: usize, slow: usize) -> Option<&'static str> {
    if fast < 2 || fast >= slow || closes.len() < slow + 1 {
        return None;
    }
    let n = closes.len();
    let avg =
        |end: usize, w: usize| closes[end - w..end].iter().copied().sum::<D>() / D::from(w as u64);
    let prev = avg(n - 1, fast) - avg(n - 1, slow);
    let current = avg(n, fast) - avg(n, slow);
    if prev <= zero() && current > zero() {
        Some("BUY")
    } else if prev >= zero() && current < zero() {
        Some("SELL")
    } else {
        Some("HOLD")
    }
}
pub fn strategy_intent(b: &Bot, q: &Quote, bar: i64) -> Intent {
    let cap = q.ask * (D::ONE + b.limits.slippage_bps / d("10000")) * (D::ONE + d("0.001"));
    Intent {
        client_id: format!("{}-{bar}", b.id),
        bot_id: b.id.clone(),
        instrument: b.instrument.clone(),
        side: Side::Buy,
        kind: Kind::Market,
        quantity: floor(b.order_eur / cap, q.lot),
        limit: None,
        stop: Some(floor(q.bid * (D::ONE - b.stop_bps / d("10000")), q.tick)),
        target: Some(ceil(q.ask * (D::ONE + b.target_bps / d("10000")), q.tick)),
        position_id: None,
    }
}

#[cfg(test)]
#[path = "risk_tests.rs"]
mod tests;
