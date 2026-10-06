//! Deterministic exchange authorization. Only this module mints execution capabilities.
use crate::order_types::*;
use anyhow::{Result, ensure};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradingLimits {
    pub account_uid: String,
    pub instruments: Vec<String>,
    pub capital: D,
    pub max_order: D,
    pub max_position: D,
    pub trade_risk: D,
    pub daily_loss: D,
    pub total_loss: D,
    pub drawdown: D,
    pub max_orders: usize,
    pub orders_per_minute: usize,
    pub spread_bps: D,
    pub confirmed: bool,
}
impl TradingLimits {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.confirmed && !self.account_uid.is_empty() && self.account_uid.len() <= 80,
            "Bevestig het gedetecteerde account en alle limieten"
        );
        ensure!(
            !self.instruments.is_empty()
                && self.instruments.len() <= 2000
                && self
                    .instruments
                    .iter()
                    .all(|s| (s.ends_with("-EUR") || s.ends_with("-USDC")) && s.split_once('-').is_some_and(|(base, quote)| !base.is_empty() && !quote.is_empty() && !quote.contains('-'))
                        && s.len() <= 40
                        && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')),
            "Bevestig de beschikbare spotmarkten (maximaal 2000)"
        );
        for value in [
            self.capital,
            self.max_order,
            self.max_position,
            self.trade_risk,
            self.daily_loss,
            self.total_loss,
            self.drawdown,
            self.spread_bps,
        ] {
            positive(value)?;
        }
        ensure!(
            self.max_order <= self.capital
                && self.max_position <= self.capital
                && self.trade_risk <= self.capital
                && self.daily_loss <= self.capital
                && self.total_loss <= self.capital
                && self.drawdown <= self.capital,
            "Limieten mogen het bevestigde handelsbudget niet overschrijden"
        );
        ensure!(
            (1..=100).contains(&self.max_orders)
                && (1..=60).contains(&self.orders_per_minute)
                && self.spread_bps <= D::from(500),
            "Ongeldige order-, frequentie- of spreadlimiet"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionQuote {
    pub bid: D,
    pub ask: D,
    pub received_ms: i64,
    pub rules: MarketRules,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountSnapshot {
    pub uid: String,
    pub mode: ExecutionMode,
    pub received_ms: i64,
    pub connected: bool,
    pub reconciled: bool,
    pub trade_permission: bool,
    pub valuation_complete: bool,
    pub equity_eur: D,
    pub available: BTreeMap<String, D>,
    pub holdings: BTreeMap<String, D>,
    pub quotes: BTreeMap<String, ExecutionQuote>,
    pub fee_rate: D,
    #[serde(default)]
    pub fee_rates: BTreeMap<String, D>,
    #[serde(default)]
    pub tradable: Vec<String>,
    pub daily_loss: D,
    pub total_loss: D,
    pub drawdown: D,
    pub native_protection_verified: bool,
    pub live_acceptance_complete: bool,
    #[serde(default)]
    pub live_enabled: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveRelease {
    pub account_uid: String,
    pub policy_revision: i64,
    pub confirmed_ms: i64,
}
pub fn live_ready(s: &AccountSnapshot, p: &TradingLimits, now: i64) -> Result<()> {
    p.validate()?;
    ensure!(
        s.mode == ExecutionMode::Live && s.uid == p.account_uid,
        "Live-account komt niet overeen met je bevestigde account"
    );
    ensure!(
        s.connected
            && s.reconciled
            && s.trade_permission
            && (0..=15000).contains(&(now - s.received_ms)),
        "Live-account, handelsrechten en private verbinding moeten actueel en gesynchroniseerd zijn"
    );
    ensure!(
        s.native_protection_verified,
        "Bevestig eerst de native bescherming en alle onzekere orders"
    );
    ensure!(
        p.instruments.iter().all(|id| s.tradable.contains(id)),
        "Bevestigde markt is niet beschikbaar op deze rekening"
    );
    Ok(())
}
/// This is a personal release for a particular policy revision, not a test result.
pub fn apply_live_release(
    s: &mut AccountSnapshot,
    p: Option<&TradingLimits>,
    release: Option<&LiveRelease>,
    revision: i64,
    now: i64,
) {
    s.live_enabled = s.mode == ExecutionMode::Live
        && p.zip(release).is_some_and(|(p, r)| {
            r.account_uid == s.uid
                && r.policy_revision == revision
                && r.confirmed_ms > 0
                && r.confirmed_ms <= now
                && live_ready(s, p, now).is_ok()
        });
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub instrument: String,
    pub side: Direction,
    pub quantity: D,
    pub reserve: D,
    pub created_ms: i64,
    pub active: bool,
}
pub fn claim_base(c: &Claim) -> &str {
    c.instrument
        .rsplit_once('-')
        .map(|(base, _)| base)
        .unwrap_or("")
}
pub fn claim_quote(c: &Claim) -> &str {
    c.instrument
        .rsplit_once('-')
        .map(|(_, quote)| quote)
        .unwrap_or("")
}
pub fn claim_currency(c: &Claim) -> &str {
    if c.side == Direction::Buy { claim_quote(c) } else { claim_base(c) }
}
pub fn claim_amount(c: &Claim) -> D {
    if c.side == Direction::Buy { c.reserve } else { c.quantity }
}
pub fn account_fee(s: &AccountSnapshot, instrument: &str) -> Result<D> {
    // Older persisted EUR snapshots remain readable. New snapshots require a fee
    // checked for this exact instrument, including incentive/fee-group differences.
    if s.fee_rates.is_empty() && instrument.ends_with("-EUR") { return Ok(s.fee_rate); }
    s.fee_rates.get(instrument).copied().ok_or_else(|| anyhow::anyhow!("Actuele accountfee voor deze markt ontbreekt; wacht op marktcontrole"))
}
pub fn euro_bounds(s: &AccountSnapshot, currency: &str, now: i64) -> Result<(D,D)> {
    fn walk(s: &AccountSnapshot, c: &str, now: i64, path: &mut Vec<String>) -> Option<(D,D)> {
        if c == "EUR" { return Some((D::ONE,D::ONE)); }
        if path.len() >= 3 || path.iter().any(|v| v == c) { return None; }
        path.push(c.into());
        // Direct EUR paths first. Otherwise use actual conversion books; never
        // assume USD, USDC or USDT are equal to each other or to one EUR.
        let mut quotes: Vec<_> = s.quotes.values().collect();
        quotes.sort_by_key(|q| q.rules.quote != "EUR" && q.rules.base != "EUR");
        for q in quotes {
            if !(0..=15000).contains(&(now-q.received_ms)) || q.bid <= D::ZERO || q.ask < q.bid { continue; }
            let edge = if q.rules.base == c {
                Some((q.rules.quote.as_str(),q.bid,q.ask))
            } else if q.rules.quote == c {
                Some((q.rules.base.as_str(),D::ONE.checked_div(q.ask)?,D::ONE.checked_div(q.bid)?))
            } else { None };
            if let Some((next,lo,hi)) = edge {
                if let Some((a,b)) = walk(s,next,now,path) {
                    path.pop();return Some((lo.checked_mul(a)?,hi.checked_mul(b)?));
                }
            }
        }
        path.pop();None
    }
    walk(s,currency,now,&mut vec![]).ok_or_else(||anyhow::anyhow!("Actuele EUR-waardering ontbreekt voor {}",currency))
}
fn euro_rate(s: &AccountSnapshot, currency: &str, now: i64) -> Result<D> { Ok(euro_bounds(s,currency,now)?.1) }
#[derive(Debug)]
pub struct AuthorizedOrder {
    compiled: CompiledOrder,
    reserve_currency: String,
    reserve: D,
    expires_ms: i64,
}
impl AuthorizedOrder {
    pub fn compiled(&self) -> &CompiledOrder {
        &self.compiled
    }
    pub fn currency(&self) -> &str {
        &self.reserve_currency
    }
    pub fn reserve(&self) -> D {
        self.reserve
    }
    pub fn expires_ms(&self) -> i64 {
        self.expires_ms
    }
    pub fn payload(&self) -> &Value {
        &self.compiled.payload
    }
}
/// Reducing native protection, minted only from reconciled fill quantities.
pub fn authorize_protection(
    parent: &Ticket,
    rules: &MarketRules,
    net: D,
    committed: D,
    id: &str,
    now: i64,
) -> Result<AuthorizedOrder> {
    ensure!(
        parent.side == Direction::Buy,
        "Alleen verworven spotinventaris beschermen"
    );
    let protection = parent
        .protection
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Beschermende stop ontbreekt"))?;
    ensure!(
        rules.lot > D::ZERO && rules.minimum > D::ZERO,
        "Ongeldige instrumentregels voor bescherming"
    );
    ensure!(
        committed >= D::ZERO && committed < net,
        "Geen nieuwe beschermbare inventaris"
    );
    let quantity = ((net - committed) / rules.lot).floor() * rules.lot;
    ensure!(
        quantity >= rules.minimum,
        "Fill onder OKX-minimum; geen fictieve bescherming"
    );
    let ticket = Ticket {
        mode: parent.mode,
        instrument: parent.instrument.clone(),
        side: Direction::Sell,
        quantity,
        order: OrderSpec::TpSl {
            take_profit: protection.target.map(|trigger| StopLeg {
                trigger,
                order_price: None,
            }),
            stop_loss: Some(StopLeg {
                trigger: protection.stop,
                order_price: None,
            }),
        },
        protection: None,
    };
    let compiled = ticket.compile(rules, protection.stop, id)?;
    Ok(AuthorizedOrder {
        compiled,
        reserve_currency: rules.base.clone(),
        reserve: quantity,
        expires_ms: now + 3000,
    })
}
pub fn scoped_bot_snapshot(account:&AccountSnapshot,report:&Value)->Result<AccountSnapshot>{
 let mut own=account.clone();
 let number=|k:&str|->Result<D>{Ok(report[k].as_str().ok_or_else(||anyhow::anyhow!("Botwaardering ontbreekt"))?.parse()?)};
 own.holdings=serde_json::from_value(report["holdings"].clone())?;own.equity_eur=number("equity_eur")?;own.daily_loss=number("daily_loss_eur")?;own.total_loss=number("total_loss_eur")?;own.drawdown=number("drawdown_eur")?;
 Ok(own)
}
/// A reducing capability for ledger-owned inventory. It survives revoked entry
/// permissions and losses, but requires fresh reconciled balances and all claims.
pub fn authorize_owned_reduction(ticket:&Ticket,account_uid:&str,s:&AccountSnapshot,claims:&[Claim],owned:D,now:i64,id:&str)->Result<AuthorizedOrder>{
 ensure!(ticket.side==Direction::Sell&&ticket.mode==s.mode&&account_uid==s.uid&&ticket.protection.is_none()&&matches!(ticket.order,OrderSpec::Limit{policy:LimitPolicy::Ioc,..}),"Alleen prijsbegrensde verkoop van eigen spotinventaris");
 ensure!(s.connected&&s.reconciled&&s.trade_permission&&(0..=15000).contains(&(now-s.received_ms)),"Afsluiten wacht op actuele private reconciliatie");
 let q=s.quotes.get(&ticket.instrument).ok_or_else(||anyhow::anyhow!("Afsluitmarkt ontbreekt"))?;
 ensure!((0..=5000).contains(&(now-q.received_ms))&&q.bid>D::ZERO&&q.ask>=q.bid,"Afsluitprijs ontbreekt of is verouderd");
 ensure!(ticket.quantity>D::ZERO&&ticket.quantity<=owned,"Afsluiten kan geen inventaris van een andere bot gebruiken");
 let other:D=claims.iter().filter(|c|c.active&&c.id!=id&&claim_currency(c)==q.rules.base).map(claim_amount).sum();
 ensure!(ticket.quantity+other<=s.available.get(&q.rules.base).copied().unwrap_or_default(),"Inventaris is reeds gereserveerd");
 Ok(AuthorizedOrder{compiled:ticket.compile(&q.rules,q.bid,id)?,reserve_currency:q.rules.base.clone(),reserve:ticket.quantity,expires_ms:now+3000})
}
pub fn authorize(
    ticket: &Ticket,
    policy: &TradingLimits,
    snapshot: &AccountSnapshot,
    claims: &[Claim],
    now: i64,
    client_id: &str,
) -> Result<AuthorizedOrder> {
    policy.validate()?;
    ensure!(
        ticket.mode == snapshot.mode && policy.account_uid == snapshot.uid,
        "Account of omgeving gewijzigd; nieuwe accountcontrole vereist"
    );
    ensure!(
        snapshot.connected
            && snapshot.reconciled
            && snapshot.trade_permission
            && (0..=15000).contains(&(now - snapshot.received_ms)),
        "OKX-rekening ontbreekt, is verouderd of nog niet gesynchroniseerd"
    );
    ensure!(
        ticket.mode != ExecutionMode::Live || snapshot.live_enabled,
        "Live nog niet vrijgegeven voor dit account en deze handelslimieten"
    );
    ensure!(
        policy.instruments.contains(&ticket.instrument),
        "Instrument is niet persoonlijk bevestigd"
    );
    let quote = snapshot
        .quotes
        .get(&ticket.instrument)
        .ok_or_else(|| anyhow::anyhow!("Geen accounttoegankelijke spotmarkt"))?;
    ensure!(
        (0..=5000).contains(&(now - quote.received_ms))
            && quote.bid > D::ZERO
            && quote.ask >= quote.bid,
        "Marktprijs ontbreekt of is verouderd"
    );
    let compiled = ticket.compile(
        &quote.rules,
        if ticket.side == Direction::Buy {
            quote.ask
        } else {
            quote.bid
        },
        client_id,
    )?;
    let recent = claims
        .iter()
        .filter(|c| c.id != client_id && (0..60000).contains(&(now - c.created_ms)))
        .count();
    let other: Vec<_> = claims
        .iter()
        .filter(|c| c.id != client_id && c.active)
        .collect();
    ensure!(
        other.len() < policy.max_orders,
        "Limiet voor open orders bereikt"
    );
    ensure!(recent < policy.orders_per_minute, "Orderfrequentie bereikt");
    ensure!(snapshot.tradable.contains(&ticket.instrument), "Deze spotmarkt is niet beschikbaar voor dit OKX-account");
    let fee = account_fee(snapshot, &ticket.instrument)?;
    ensure!(
        fee >= D::ZERO && fee <= D::new(1, 1),
        "Actuele fee ontbreekt of is ongeldig"
    );
    let (currency, reserve) = if ticket.side == Direction::Buy {
        let fx = euro_rate(snapshot, &quote.rules.quote, now)?;
        ensure!(
            snapshot.valuation_complete && snapshot.equity_eur > D::ZERO,
            "Rekeningwaardering onvolledig; nieuwe koop geblokkeerd"
        );
        ensure!(
            snapshot.daily_loss < policy.daily_loss
                && snapshot.total_loss < policy.total_loss
                && snapshot.drawdown < policy.drawdown,
            "Rekeningverlieslimiet bereikt"
        );
        ensure!(
            (quote.ask - quote.bid) / quote.bid * D::from(10000) <= policy.spread_bps,
            "Spreadlimiet bereikt"
        );
        let protection = ticket
            .protection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Stop-loss verplicht voor nieuwe spotblootstelling"))?;
        ensure!(
            snapshot.native_protection_verified,
            "Native bescherming voor gedeeltelijke fills is nog niet bij OKX geaccepteerd"
        );
        ensure!(
            !matches!(
                ticket.order,
                OrderSpec::Trigger {
                    execution: ExecutionPrice::Market,
                    ..
                } | OrderSpec::Trailing { .. }
                    | OrderSpec::TpSl { .. }
            ),
            "Deze koop heeft geen vaste maximale uitvoeringsprijs. Gebruik een trigger met limiet, of gebruik dit type voor verkoop van bestaande inventaris."
        );
        let amount = ticket.quantity * compiled.worst_price * (D::ONE + fee);
        let trade_risk = (ticket.quantity * (compiled.worst_price - protection.stop)
            + ticket.quantity * (compiled.worst_price + protection.stop) * fee)
            * fx;
        ensure!(
            trade_risk > D::ZERO && trade_risk <= policy.trade_risk,
            "Risico per order inclusief fee overschrijdt limiet"
        );
        ensure!(
            amount * fx <= policy.max_order,
            "Maximaal orderbedrag overschreden"
        );
        let reserved: D =
            other
                .iter()
                .filter(|c| c.side == Direction::Buy)
                .try_fold(D::ZERO, |sum, c| {
                    Ok::<_, anyhow::Error>(
                        sum + c.reserve * euro_rate(snapshot, claim_quote(c), now)?,
                    )
                })?;
        let pending: D = other
            .iter()
            .filter(|c| c.side == Direction::Buy && claim_base(c) == quote.rules.base)
            .try_fold(D::ZERO, |sum, c| {
                Ok::<_, anyhow::Error>(sum + c.reserve * euro_rate(snapshot, claim_quote(c), now)?)
            })?;
        let held = snapshot
            .holdings
            .get(&quote.rules.base)
            .copied()
            .unwrap_or_default()
            * quote.ask
            * fx;
        let total_held: D = snapshot.holdings.iter()
            .filter(|(base,qty)| **qty > D::ZERO && !["EUR","USDC","USDT","USD","AUD","AED","BRL","TRY","SGD","PLN"].contains(&base.as_str()))
            .try_fold(D::ZERO,|sum,(base,qty)| Ok::<_,anyhow::Error>(sum + *qty * euro_rate(snapshot,base,now)?))?;
        ensure!(
            amount * fx + reserved + total_held <= policy.capital
                && amount * fx + pending + held <= policy.max_position,
            "Rekeningbudget of muntblootstelling overschreden"
        );
        (quote.rules.quote.clone(), amount)
    } else {
        let reserved: D = other
            .iter()
            .filter(|c| claim_currency(c) == quote.rules.base)
            .map(|c| claim_amount(c))
            .sum();
        (quote.rules.base.clone(), ticket.quantity + reserved)
    };
    let available = snapshot
        .available
        .get(&currency)
        .copied()
        .unwrap_or_default();
    if ticket.side == Direction::Buy {
        // available balance already excludes OKX-frozen regular orders. Only local claims
        // may make this check conservative; never assume an algo freezes assets.
        let reserved: D = other
            .iter()
            .filter(|c| claim_currency(c) == currency)
            .map(|c| claim_amount(c))
            .sum();
        ensure!(
            reserve + reserved <= available,
            "Onvoldoende vrij saldo na reserveringen"
        );
    } else {
        ensure!(
            reserve <= available,
            "Onvoldoende vrij cryptosaldo na verkoopreserveringen"
        );
    }
    Ok(AuthorizedOrder {
        compiled,
        reserve_currency: currency,
        reserve: if ticket.side == Direction::Sell {
            ticket.quantity
        } else {
            reserve
        },
        expires_ms: now + 3000,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Ticket, TradingLimits, AccountSnapshot) {
        let ticket = Ticket {
            mode: ExecutionMode::OkxDemo,
            instrument: "BTC-EUR".into(),
            side: Direction::Buy,
            quantity: D::ONE,
            order: OrderSpec::Limit {
                price: D::from(100),
                policy: LimitPolicy::Limit,
            },
            protection: Some(Protection {
                stop: D::from(90),
                target: None,
            }),
        };
        let limits = TradingLimits {
            account_uid: "demo-account".into(),
            instruments: vec!["BTC-EUR".into()],
            capital: D::from(500),
            max_order: D::from(150),
            max_position: D::from(200),
            trade_risk: D::from(20),
            daily_loss: D::from(50),
            total_loss: D::from(100),
            drawdown: D::from(100),
            max_orders: 10,
            orders_per_minute: 10,
            spread_bps: D::from(200),
            confirmed: true,
        };
        let quote = ExecutionQuote {
            bid: D::from(99),
            ask: D::from(100),
            received_ms: 10000,
            rules: MarketRules {
                tick: D::ONE,
                lot: D::ONE,
                minimum: D::ONE,
                base: "BTC".into(),
                quote: "EUR".into(),
            },
        };
        let snapshot = AccountSnapshot {
            uid: "demo-account".into(),
            mode: ExecutionMode::OkxDemo,
            received_ms: 10000,
            connected: true,
            reconciled: true,
            trade_permission: true,
            valuation_complete: true,
            equity_eur: D::from(500),
            available: [("EUR".into(), D::from(500)), ("BTC".into(), D::from(2))]
                .into_iter()
                .collect(),
            holdings: BTreeMap::new(),
            tradable: vec!["BTC-EUR".into()],
            quotes: [("BTC-EUR".into(), quote)].into_iter().collect(),
            fee_rate: D::new(1, 3),
            fee_rates: BTreeMap::new(),
            daily_loss: D::ZERO,
            total_loss: D::ZERO,
            drawdown: D::ZERO,
            native_protection_verified: true,
            live_acceptance_complete: false,
            live_enabled: false,
        };
        (ticket, limits, snapshot)
    }
    #[test]
    fn owned_exit_survives_entry_release_loss_but_never_borrows_inventory(){
        let (mut t,l,mut s)=fixture();t.side=Direction::Sell;t.protection=None;t.order=OrderSpec::Limit{price:D::from(99),policy:LimitPolicy::Ioc};t.mode=ExecutionMode::Live;s.mode=ExecutionMode::Live;s.live_enabled=false;s.native_protection_verified=false;s.daily_loss=l.daily_loss;
        assert!(authorize_owned_reduction(&t,&s.uid,&s,&[],D::ONE,10000,"owned").is_ok());
        assert!(authorize_owned_reduction(&t,&s.uid,&s,&[],D::new(9,1),10000,"owned").is_err());
        let claim=Claim{id:"another".into(),instrument:t.instrument.clone(),side:Direction::Sell,quantity:D::from(2),reserve:D::from(2),created_ms:10000,active:true};
        assert!(authorize_owned_reduction(&t,&s.uid,&s,&[claim],D::ONE,10000,"owned").is_err());
        s.reconciled=false;assert!(authorize_owned_reduction(&t,&s.uid,&s,&[],D::ONE,10000,"owned").is_err());
    }
    #[test]
    fn live_cannot_be_enabled_by_client_mode_or_flags() {
        let (mut t, l, mut s) = fixture();
        t.mode = ExecutionMode::Live;
        s.mode = ExecutionMode::Live;
        assert!(
            authorize(&t, &l, &s, &[], 10000, "id")
                .unwrap_err()
                .to_string()
                .contains("Live nog niet vrijgegeven")
        );
    }
    #[test]
    fn release_is_bound_to_account_revision_and_current_private_readiness() {
        let (mut t, l, mut s) = fixture();
        t.mode = ExecutionMode::Live;
        s.mode = ExecutionMode::Live;
        let r = LiveRelease {
            account_uid: s.uid.clone(),
            policy_revision: 4,
            confirmed_ms: 9000,
        };
        apply_live_release(&mut s, Some(&l), Some(&r), 4, 10000);
        assert!(s.live_enabled);
        assert!(!s.live_acceptance_complete);
        assert!(authorize(&t, &l, &s, &[], 10000, "liveid").is_ok());
        apply_live_release(&mut s, Some(&l), Some(&r), 5, 10000);
        assert!(!s.live_enabled);
        apply_live_release(&mut s, Some(&l), None, 4, 10000);
        assert!(!s.live_enabled);
        s.trade_permission = false;
        apply_live_release(&mut s, Some(&l), Some(&r), 4, 10000);
        assert!(!s.live_enabled);
        s.trade_permission = true;
        s.reconciled = false;
        apply_live_release(&mut s, Some(&l), Some(&r), 4, 10000);
        assert!(!s.live_enabled);
        s.reconciled = true;
        s.uid = "other-account".into();
        apply_live_release(&mut s, Some(&l), Some(&r), 4, 10000);
        assert!(!s.live_enabled);
        s.uid = l.account_uid.clone();
        apply_live_release(&mut s, Some(&l), Some(&r), 4, 30000);
        assert!(!s.live_enabled);
    }
    #[test]
    fn usdc_buys_use_native_reserve_but_eur_limits_and_current_fx() {
        let (mut t, mut l, mut s) = fixture();
        t.instrument = "BTC-USDC".into();
        l.instruments.push(t.instrument.clone());
        s.tradable.push(t.instrument.clone());
        s.fee_rates.insert(t.instrument.clone(),s.fee_rate);
        let mut q = s.quotes["BTC-EUR"].clone();
        q.rules.quote = "USDC".into();
        s.quotes.insert(t.instrument.clone(), q);
        let mut fx = s.quotes["BTC-EUR"].clone();
        fx.rules.base = "USDC".into();
        fx.bid = D::new(8, 1);
        fx.ask = D::new(8, 1);
        s.quotes.insert("USDC-EUR".into(), fx);
        s.available.insert("USDC".into(), D::from(500));
        l.max_order = D::from(90);
        let a = authorize(&t, &l, &s, &[], 10000, "usdcfixture").unwrap();
        assert_eq!(a.currency(), "USDC");
        assert_eq!(a.reserve(), D::new(1001, 1));
        l.max_order = D::from(80);
        assert!(authorize(&t, &l, &s, &[], 10000, "usdcfixture").is_err());
        l.max_order = D::from(90);
        s.quotes.get_mut("USDC-EUR").unwrap().received_ms = 0;
        assert!(authorize(&t, &l, &s, &[], 20000, "usdcfixture").is_err());
    }
    #[test]
    fn sell_inventory_is_shared_between_eur_and_usdc_markets() {
        let (mut t, l, s) = fixture();
        t.side = Direction::Sell;
        t.protection = None;
        let c = Claim {
            id: "othermarket".into(),
            instrument: "BTC-USDC".into(),
            side: Direction::Sell,
            quantity: D::from(2),
            reserve: D::from(2),
            created_ms: 10000,
            active: true,
        };
        assert!(authorize(&t, &l, &s, &[c], 10000, "newid").is_err());
    }
    #[test]
    fn missing_protection_and_stale_or_unreconciled_state_fail() {
        let (mut t, l, mut s) = fixture();
        t.protection = None;
        assert!(authorize(&t, &l, &s, &[], 10000, "id").is_err());
        t.protection = Some(Protection {
            stop: D::from(90),
            target: None,
        });
        s.reconciled = false;
        assert!(authorize(&t, &l, &s, &[], 10000, "id").is_err());
        s.reconciled = true;
        assert!(authorize(&t, &l, &s, &[], 30000, "id").is_err());
    }
    #[test]
    fn total_parent_quantity_reserved_for_iceberg_not_one_slice() {
        let (mut t, mut l, s) = fixture();
        t.order = OrderSpec::Iceberg {
            slice: D::new(1, 1),
            visible_orders: 2,
            aggressiveness: Aggressiveness::Mid,
            price_limit: D::from(100),
            start: IcebergStart::Instant,
        };
        let mut s = s;
        s.quotes.get_mut("BTC-EUR").unwrap().rules.lot = D::new(1, 1);
        s.quotes.get_mut("BTC-EUR").unwrap().rules.minimum = D::new(1, 1);
        l.max_order = D::from(50);
        assert!(authorize(&t, &l, &s, &[], 10000, "id").is_err());
        l.max_order = D::from(150);
        assert!(authorize(&t, &l, &s, &[], 10000, "id").unwrap().reserve() > D::from(100));
    }
    #[test]
    fn simultaneous_sell_claims_never_double_spend_inventory() {
        let (mut t, l, s) = fixture();
        t.side = Direction::Sell;
        t.protection = None;
        t.quantity = D::from(2);
        let claims = [Claim {
            id: "another".into(),
            instrument: t.instrument.clone(),
            side: Direction::Sell,
            quantity: D::ONE,
            reserve: D::ONE,
            created_ms: 10000,
            active: true,
        }];
        assert!(authorize(&t, &l, &s, &claims, 10000, "id").is_err());
        assert!(authorize(&t, &l, &s, &[], 10000, "id").is_ok());
    }
    #[test]
    fn entries_fail_closed_on_losses_and_unprotected_fills() {
        let (t, l, mut s) = fixture();
        s.daily_loss = l.daily_loss;
        assert!(authorize(&t, &l, &s, &[], 10000, "id").is_err());
        s.daily_loss = D::ZERO;
        s.native_protection_verified = false;
        assert!(authorize(&t, &l, &s, &[], 10000, "id").is_err());
    }
    #[test]
    fn markets_are_not_limited_to_twelve_and_syntax_stays_spot_only() {
        let (_,mut l,_)=fixture();l.instruments=(0..500).map(|i|format!("ASSET{i}-USDC")).collect();assert!(l.validate().is_ok());
        l.instruments.push("BTC-USDT-SWAP".into());assert!(l.validate().is_err());
    }
    #[test]
    fn different_instruments_spending_the_same_currency_share_claims() {
        let (t,l,s)=fixture();let c=Claim{id:"sellcash".into(),instrument:"EUR-USDC".into(),side:Direction::Sell,quantity:D::from(450),reserve:D::from(450),created_ms:10000,active:true};
        assert!(authorize(&t,&l,&s,&[c],10000,"buybitcoin").is_err());
        let (mut t,l,s)=fixture();t.side=Direction::Sell;t.protection=None;
        let c=Claim{id:"buywithbtc".into(),instrument:"ETH-BTC".into(),side:Direction::Buy,quantity:D::ONE,reserve:D::from(2),created_ms:10000,active:true};
        assert!(authorize(&t,&l,&s,&[c],10000,"sellbitcoin").is_err());
    }
    #[test]
    fn inverse_conversion_and_stale_fx_do_not_assume_stablecoin_parity() {
        let (_,_,mut s)=fixture();let mut q=s.quotes["BTC-EUR"].clone();q.bid=D::from(90);q.ask=D::from(100);s.quotes.insert("BTC-EUR".into(),q.clone());q.rules.quote="USDT".into();q.bid=D::from(180);q.ask=D::from(200);s.quotes.insert("BTC-USDT".into(),q);
        let (bid,ask)=euro_bounds(&s,"USDT",10000).unwrap();assert_eq!(bid,D::new(45,2));assert!((ask-D::from(100)/D::from(180)).abs()<D::new(1,25));
        assert!(euro_bounds(&s,"USDT",30000).is_err());assert!(euro_bounds(&s,"AUD",10000).is_err());
    }
    #[test]
    fn new_snapshots_need_fees_for_the_exact_selected_market() {
        let (t,l,mut s)=fixture();s.fee_rates.insert("ETH-EUR".into(),D::ZERO);assert!(authorize(&t,&l,&s,&[],10000,"feechecked").is_err());s.fee_rates.insert(t.instrument.clone(),D::new(1,3));assert!(authorize(&t,&l,&s,&[],10000,"feechecked").is_ok());s.tradable.clear();assert!(authorize(&t,&l,&s,&[],10000,"feechecked").is_err());
    }

}
