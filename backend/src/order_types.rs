//! Typed, credential-free spot order contract shared by terminal and gateway.
use anyhow::{Result, ensure};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionMode {
    OkxDemo,
    Live,
}
impl ExecutionMode {
    pub fn id(self) -> &'static str {
        match self {
            Self::OkxDemo => "OKX_DEMO",
            Self::Live => "LIVE",
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Buy,
    Sell,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LimitPolicy {
    Limit,
    PostOnly,
    Ioc,
    Fok,
}
impl LimitPolicy {
    fn id(self) -> &'static str {
        match self {
            Self::Limit => "limit",
            Self::PostOnly => "post_only",
            Self::Ioc => "ioc",
            Self::Fok => "fok",
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Aggressiveness {
    Radical,
    Mid,
    Conservative,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "unit", content = "value", rename_all = "snake_case")]
pub enum Distance {
    Ratio(D),
    Price(D),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExecutionPrice {
    Market,
    Limit { price: D, policy: LimitPolicy },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StopLeg {
    pub trigger: D,
    pub order_price: Option<D>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum IcebergStart {
    Instant,
    Price {
        price: D,
        condition: String,
    },
    Rsi {
        timeframe: String,
        threshold: u8,
        condition: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum OrderSpec {
    Market {
        slippage_bps: u16,
    },
    Limit {
        price: D,
        policy: LimitPolicy,
    },
    Trigger {
        trigger: D,
        execution: ExecutionPrice,
    },
    TpSl {
        take_profit: Option<StopLeg>,
        stop_loss: Option<StopLeg>,
    },
    Trailing {
        distance: Distance,
        activation: Option<D>,
    },
    Iceberg {
        slice: D,
        visible_orders: u16,
        aggressiveness: Aggressiveness,
        price_limit: D,
        start: IcebergStart,
    },
    Twap {
        slice: D,
        price_limit: D,
        interval_seconds: u32,
        variance: Distance,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Protection {
    pub stop: D,
    pub target: Option<D>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Ticket {
    pub mode: ExecutionMode,
    pub instrument: String,
    pub side: Direction,
    pub quantity: D,
    pub order: OrderSpec,
    pub protection: Option<Protection>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketRules {
    pub tick: D,
    pub lot: D,
    pub minimum: D,
    pub base: String,
    pub quote: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompiledOrder {
    pub endpoint: String,
    pub payload: Value,
    pub algo: bool,
    pub kind: String,
    pub worst_price: D,
}
pub fn positive(v: D) -> Result<()> {
    ensure!(
        v > D::ZERO && v <= D::from(1_000_000_000_u64),
        "Waarde moet positief zijn en binnen bereik liggen"
    );
    Ok(())
}
fn tick(v: D, step: D) -> Result<()> {
    positive(v)?;
    ensure!(
        step > D::ZERO && v % step == D::ZERO,
        "Prijs voldoet niet aan de OKX-prijsstap"
    );
    Ok(())
}
fn distance(v: &Distance, market: &MarketRules, twap: bool) -> Result<()> {
    match v {
        Distance::Ratio(r) => {
            positive(*r)?;
            let max = if twap { D::new(1, 2) } else { D::ONE };
            ensure!(
                *r <= max && (!twap || *r >= D::new(1, 4)),
                "Percentage buiten het OKX-bereik"
            );
        }
        Distance::Price(p) => {
            if twap && *p == D::ZERO {
                return Ok(());
            }
            tick(*p, market.tick)?;
        }
    }
    Ok(())
}
impl Ticket {
    pub fn compile(
        &self,
        market: &MarketRules,
        reference: D,
        client_id: &str,
    ) -> Result<CompiledOrder> {
        ensure!(
            !market.base.is_empty() && ["EUR","USDC"].contains(&market.quote.as_str()) && self.instrument == format!("{}-{}", market.base, market.quote),
            "Dit uitvoeringsprofiel ondersteunt accounttoegankelijke spotmarkten zonder hefboom"
        );
        ensure!(
            !client_id.is_empty()
                && client_id.len() <= 32
                && client_id.bytes().all(|c| c.is_ascii_alphanumeric()),
            "Ongeldig client-order-id"
        );
        positive(self.quantity)?;
        positive(reference)?;
        ensure!(
            market.lot > D::ZERO
                && market.minimum > D::ZERO
                && self.quantity >= market.minimum
                && self.quantity % market.lot == D::ZERO,
            "Hoeveelheid voldoet niet aan lot/minimum"
        );
        let mut payload = json!({"instId":self.instrument,"tdMode":"cash","side":self.side,"sz":self.quantity.to_string(),"tag":"WPOSTerminal"});
        let mut algo = true;
        let worst_price;
        let kind;
        let leg = |v: &StopLeg| -> Result<()> {
            tick(v.trigger, market.tick)?;
            if let Some(p) = v.order_price {
                tick(p, market.tick)?;
            }
            Ok(())
        };
        match &self.order {
            OrderSpec::Market { slippage_bps } => {
                ensure!(*slippage_bps <= 500, "Maximale spot-market-slippage is 5%");
                payload["ordType"] = json!("market");
                payload["tgtCcy"] = json!("base_ccy");
                payload["banAmend"] = json!(true);
                // OKX accepts slippagePct only when sz is the RECEIVED currency.
                // Here sz is base quantity, so this parameter is valid only for buys.
                if self.side == Direction::Buy {
                    payload["slippagePct"] = json!(
                        (D::from(*slippage_bps) / D::from(10000))
                            .normalize()
                            .to_string()
                    );
                }
                worst_price = reference * (D::ONE + D::from(*slippage_bps) / D::from(10000));
                algo = false;
                kind = "market";
            }
            OrderSpec::Limit { price, policy } => {
                tick(*price, market.tick)?;
                payload["ordType"] = json!(policy.id());
                payload["px"] = json!(price.to_string());
                payload["pxAmendType"] = json!("0");
                worst_price = *price;
                algo = false;
                kind = policy.id();
            }
            OrderSpec::Trigger { trigger, execution } => {
                tick(*trigger, market.tick)?;
                payload["ordType"] = json!("trigger");
                payload["triggerPx"] = json!(trigger.to_string());
                payload["triggerPxType"] = json!("last");
                match execution {
                    ExecutionPrice::Market => {
                        payload["orderPx"] = json!("-1");
                        worst_price = (*trigger).max(reference);
                    }
                    ExecutionPrice::Limit { price, policy } => {
                        tick(*price, market.tick)?;
                        ensure!(
                            *policy != LimitPolicy::PostOnly,
                            "OKX trigger ondersteunt limiet, IOC of FOK; geen post-only"
                        );
                        payload["orderPx"] = json!(price.to_string());
                        if *policy != LimitPolicy::Limit {
                            payload["advanceOrdType"] = json!(policy.id());
                        }
                        worst_price = *price;
                    }
                }
                kind = "trigger";
            }
            OrderSpec::TpSl {
                take_profit,
                stop_loss,
            } => {
                ensure!(
                    take_profit.is_some() || stop_loss.is_some(),
                    "Vul TP en/of SL in"
                );
                payload["ordType"] = json!(if take_profit.is_some() && stop_loss.is_some() {
                    "oco"
                } else {
                    "conditional"
                });
                if let Some(v) = take_profit {
                    leg(v)?;
                    payload["tpTriggerPx"] = json!(v.trigger.to_string());
                    payload["tpOrdPx"] =
                        json!(v.order_price.map(|p| p.to_string()).unwrap_or("-1".into()));
                    payload["tpTriggerPxType"] = json!("last");
                }
                if let Some(v) = stop_loss {
                    leg(v)?;
                    payload["slTriggerPx"] = json!(v.trigger.to_string());
                    payload["slOrdPx"] =
                        json!(v.order_price.map(|p| p.to_string()).unwrap_or("-1".into()));
                    payload["slTriggerPxType"] = json!("last");
                }
                worst_price = reference
                    .max(
                        take_profit
                            .as_ref()
                            .map(|v| v.order_price.unwrap_or(v.trigger))
                            .unwrap_or(reference),
                    )
                    .max(
                        stop_loss
                            .as_ref()
                            .map(|v| v.order_price.unwrap_or(v.trigger))
                            .unwrap_or(reference),
                    );
                kind = if take_profit.is_some() && stop_loss.is_some() {
                    "oco"
                } else {
                    "conditional"
                };
            }
            OrderSpec::Trailing {
                distance: trail,
                activation,
            } => {
                distance(trail, market, false)?;
                payload["ordType"] = json!("move_order_stop");
                match trail {
                    Distance::Ratio(v) => payload["callbackRatio"] = json!(v.to_string()),
                    Distance::Price(v) => payload["callbackSpread"] = json!(v.to_string()),
                }
                if let Some(p) = activation {
                    tick(*p, market.tick)?;
                    payload["activePx"] = json!(p.to_string());
                }
                worst_price = reference.max(activation.unwrap_or(reference));
                kind = "move_order_stop";
            }
            OrderSpec::Iceberg {
                slice,
                visible_orders,
                aggressiveness,
                price_limit,
                start,
            } => {
                self.slice(*slice, market)?;
                tick(*price_limit, market.tick)?;
                ensure!(
                    (1..=100).contains(visible_orders),
                    "Aantal zichtbare delen moet tussen 1 en 100 liggen; accountlimieten kunnen strenger zijn"
                );
                payload["ordType"] = json!("smart_iceberg");
                payload["szLimit"] = json!(slice.to_string());
                payload["lmtOrderNumber"] = json!(visible_orders.to_string());
                payload["aggressiveness"] = json!(aggressiveness);
                payload["pxLimit"] = json!(price_limit.to_string());
                let mut t = json!({"triggerAction":"start"});
                match start {
                    IcebergStart::Instant => t["triggerStrategy"] = json!("instant"),
                    IcebergStart::Price { price, condition } => {
                        tick(*price, market.tick)?;
                        ensure!(
                            ["cross_up", "cross_down"].contains(&condition.as_str()),
                            "Ongeldige prijsvoorwaarde"
                        );
                        t["triggerStrategy"] = json!("price");
                        t["triggerPx"] = json!(price.to_string());
                        t["triggerCond"] = json!(condition);
                    }
                    IcebergStart::Rsi {
                        timeframe,
                        threshold,
                        condition,
                    } => {
                        ensure!(
                            ["3m", "5m", "15m", "30m", "1H", "4H", "1D"]
                                .contains(&timeframe.as_str())
                                && (1..=100).contains(threshold)
                                && ["cross_up", "cross_down", "above", "below", "cross"]
                                    .contains(&condition.as_str()),
                            "Ongeldige RSI-startvoorwaarde"
                        );
                        t["triggerStrategy"] = json!("rsi");
                        t["timeframe"] = json!(timeframe);
                        t["thold"] = json!(threshold.to_string());
                        t["timePeriod"] = json!("14");
                        t["triggerCond"] = json!(condition);
                    }
                }
                payload["triggerParams"] = json!([t]);
                worst_price = *price_limit;
                kind = "smart_iceberg";
            }
            OrderSpec::Twap {
                slice,
                price_limit,
                interval_seconds,
                variance,
            } => {
                self.slice(*slice, market)?;
                tick(*price_limit, market.tick)?;
                ensure!(
                    (1..=86400).contains(interval_seconds),
                    "Ongeldig interval in seconden"
                );
                distance(variance, market, true)?;
                payload["ordType"] = json!("twap");
                payload["szLimit"] = json!(slice.to_string());
                payload["pxLimit"] = json!(price_limit.to_string());
                payload["timeInterval"] = json!(interval_seconds.to_string());
                match variance {
                    Distance::Ratio(v) => payload["pxVar"] = json!(v.to_string()),
                    Distance::Price(v) => payload["pxSpread"] = json!(v.to_string()),
                }
                worst_price = *price_limit;
                kind = "twap";
            }
        }
        payload[if algo { "algoClOrdId" } else { "clOrdId" }] = json!(client_id);
        if let Some(protection) = &self.protection {
            ensure!(
                self.side == Direction::Buy,
                "Bescherming bij een entry is alleen voor een spotkoop"
            );
            tick(protection.stop, market.tick)?;
            ensure!(
                protection.stop < worst_price,
                "Stop moet onder de instap liggen"
            );
            if let Some(p) = protection.target {
                tick(p, market.tick)?;
                ensure!(p > worst_price, "Target moet boven de instap liggen");
            }
            // Protection is submitted for confirmed, fee-adjusted fills by the gateway.
            // OKX attached algos activate only after a full fill; combining both paths
            // would create overlapping sell quantities when the parent completes.
        }
        Ok(CompiledOrder {
            endpoint: if algo {
                "/api/v5/trade/order-algo"
            } else {
                "/api/v5/trade/order"
            }
            .into(),
            payload,
            algo,
            kind: kind.into(),
            worst_price,
        })
    }
    fn slice(&self, qty: D, market: &MarketRules) -> Result<()> {
        positive(qty)?;
        ensure!(
            qty >= market.minimum && qty < self.quantity && qty % market.lot == D::ZERO,
            "Deelgrootte moet aan lot/minimum voldoen en kleiner dan totaal zijn"
        );
        Ok(())
    }
}
pub fn catalog() -> Value {
    json!([
        {"type":"market","group":"Basis","name":"Marktorder"}, {"type":"limit","group":"Basis","name":"Limietorder"},
        {"type":"post_only","group":"Geavanceerd","name":"Post-only"}, {"type":"ioc","group":"Geavanceerd","name":"Immediate or cancel"}, {"type":"fok","group":"Geavanceerd","name":"Fill or kill"},
        {"type":"trigger","group":"Geavanceerd","name":"Trigger"}, {"type":"tp_sl","group":"Geavanceerd","name":"Take-profit / stop-loss · OCO"}, {"type":"trailing","group":"Geavanceerd","name":"Trailing stop"},
        {"type":"iceberg","group":"Gesplitst","name":"Iceberg"}, {"type":"twap","group":"Gesplitst","name":"TWAP"}
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> MarketRules {
        MarketRules {
            tick: D::new(1, 1),
            lot: D::new(1, 4),
            minimum: D::new(1, 3),
            base: "BTC".into(),
            quote: "EUR".into(),
        }
    }
    fn ticket(order: OrderSpec) -> Ticket {
        Ticket {
            mode: ExecutionMode::OkxDemo,
            instrument: "BTC-EUR".into(),
            side: Direction::Sell,
            quantity: D::ONE,
            order,
            protection: None,
        }
    }
    fn compile(t: Ticket) -> CompiledOrder {
        t.compile(&rules(), D::from(100), "WPOS123").unwrap()
    }
    #[test]
    fn native_market_qty_is_base_and_never_silently_reduced() {
        let mut t = ticket(OrderSpec::Market { slippage_bps: 100 });
        let p = compile(t.clone());
        assert_eq!(p.payload["tgtCcy"], "base_ccy");
        assert_eq!(p.payload["banAmend"], true);
        assert!(p.payload.get("slippagePct").is_none());
        t.side = Direction::Buy;
        assert_eq!(compile(t).payload["slippagePct"], "0.01");
    }
    #[test]
    fn oco_never_degrades_to_conditional_that_ignores_tp() {
        let p = compile(ticket(OrderSpec::TpSl {
            take_profit: Some(StopLeg {
                trigger: D::from(110),
                order_price: None,
            }),
            stop_loss: Some(StopLeg {
                trigger: D::from(90),
                order_price: None,
            }),
        }));
        assert_eq!(p.payload["ordType"], "oco");
        assert_eq!(p.payload["slOrdPx"], "-1");
    }
    #[test]
    fn current_iceberg_api_not_deprecated_iceberg() {
        let p = compile(ticket(OrderSpec::Iceberg {
            slice: D::new(1, 1),
            visible_orders: 3,
            aggressiveness: Aggressiveness::Conservative,
            price_limit: D::from(100),
            start: IcebergStart::Instant,
        }));
        assert_eq!(p.payload["ordType"], "smart_iceberg");
        assert_eq!(p.payload["lmtOrderNumber"], "3");
        assert_eq!(p.payload["aggressiveness"], "conservative");
        assert!(p.payload.get("pxVar").is_none());
    }
    #[test]
    fn iceberg_rsi_fields_and_fresh_trigger_execution() {
        let p = compile(ticket(OrderSpec::Iceberg {
            slice: D::new(1, 1),
            visible_orders: 1,
            aggressiveness: Aggressiveness::Mid,
            price_limit: D::from(100),
            start: IcebergStart::Rsi {
                timeframe: "15m".into(),
                threshold: 30,
                condition: "cross_down".into(),
            },
        }));
        assert_eq!(p.payload["triggerParams"][0]["timePeriod"], "14");
        let p = compile(ticket(OrderSpec::Trigger {
            trigger: D::from(90),
            execution: ExecutionPrice::Limit {
                price: D::from(89),
                policy: LimitPolicy::Fok,
            },
        }));
        assert_eq!(p.payload["advanceOrdType"], "fok");
        assert_eq!(p.payload["triggerPxType"], "last");
    }
    #[test]
    fn malformed_or_unsupported_payloads_fail() {
        assert!(serde_json::from_value::<Ticket>(json!({"mode":"LIVE","approved":true})).is_err());
        let t = ticket(OrderSpec::Limit {
            price: D::new(10001, 2),
            policy: LimitPolicy::Limit,
        });
        assert!(t.compile(&rules(), D::from(100), "ok").is_err());
        let t = ticket(OrderSpec::Trigger {
            trigger: D::from(90),
            execution: ExecutionPrice::Limit {
                price: D::from(89),
                policy: LimitPolicy::PostOnly,
            },
        });
        assert!(t.compile(&rules(), D::from(100), "ok").is_err());
    }
    #[test]
    fn slice_and_callback_bounds() {
        let t = ticket(OrderSpec::Twap {
            slice: D::ONE,
            price_limit: D::from(100),
            interval_seconds: 10,
            variance: Distance::Ratio(D::new(1, 3)),
        });
        assert!(t.compile(&rules(), D::from(100), "ok").is_err());
        let t = ticket(OrderSpec::Trailing {
            distance: Distance::Ratio(D::ZERO),
            activation: None,
        });
        assert!(t.compile(&rules(), D::from(100), "ok").is_err());
    }
    #[test]
    fn limit_policies_are_exact_and_spot_has_no_leverage_fields() {
        for policy in [
            LimitPolicy::Limit,
            LimitPolicy::PostOnly,
            LimitPolicy::Ioc,
            LimitPolicy::Fok,
        ] {
            let p = compile(ticket(OrderSpec::Limit {
                price: D::from(100),
                policy,
            }));
            assert_eq!(p.payload["ordType"], policy.id());
            assert_eq!(p.payload["tdMode"], "cash");
            assert!(p.payload.get("posSide").is_none());
            assert!(p.payload.get("reduceOnly").is_none());
        }
    }
}
