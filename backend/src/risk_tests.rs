use super::*;
const NOW: i64 = 1_700_000_000_000;
fn quote() -> Quote {
    Quote {
        instrument: "BTC-EUR".into(),
        base: "BTC".into(),
        quote: "EUR".into(),
        bid: d("100"),
        ask: d("100.1"),
        bid_size: d("100"),
        ask_size: d("100"),
        tick: d("0.01"),
        lot: d("0.01"),
        minimum: d("0.01"),
        ts: NOW,
        received_ms: NOW,
    }
}
fn account() -> Paper {
    Paper::new(Limits::preset(d("10000")), Limits::preset(d("1000")), NOW).unwrap()
}
fn intent(id: &str) -> Intent {
    Intent {
        client_id: id.into(),
        bot_id: "manual".into(),
        instrument: "BTC-EUR".into(),
        side: Side::Buy,
        kind: Kind::Market,
        quantity: d("1"),
        limit: None,
        stop: Some(d("99")),
        target: Some(d("104")),
        position_id: None,
    }
}
fn qs(q: &Quote) -> Quotes {
    [(q.instrument.clone(), q.clone())].into_iter().collect()
}
fn entered() -> (Paper, Quote, String) {
    let mut p = account();
    let q = quote();
    let id = p.submit(intent("buy"), &q, NOW).unwrap();
    p.tick(&qs(&q), NOW + 500);
    p.tick(&qs(&q), NOW + 501);
    assert_eq!(p.orders[&id].state, OrderState::Filled);
    (p, q, id)
}
#[test]
fn explicit_limits_required() {
    let mut l = Limits::preset(d("1000"));
    l.daily_loss = zero();
    assert!(l.validate().is_err());
    l.daily_loss = d("30");
    l.leverage = d("2");
    assert!(l.validate().is_err());
    l.leverage = D::ONE;
    l.ai_budget = D::ONE;
    assert!(l.validate().is_ok());
    l.ai_budget = d("101");
    assert!(l.validate().is_err());
    l.ai_budget = d("-1");
    assert!(l.validate().is_err());
}
#[test]
fn no_live_or_demo_mode() {
    for mode in ["LIVE", "OKX_DEMO", "READ_ONLY"] {
        let mut p = account();
        p.mode = mode.into();
        assert!(p.submit(intent("no"), &quote(), NOW).is_err());
    }
}
#[test]
fn unknown_intent_fields_fail() {
    let mut v = serde_json::to_value(intent("x")).unwrap();
    v["risk_approved"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Intent>(v).is_err());
}
#[test]
fn no_execution_on_preview() {
    let p = account();
    p.preview(&intent("x"), &quote(), NOW).unwrap();
    assert!(p.orders.is_empty());
    assert_eq!(p.reserve(None), zero());
}
#[test]
fn reserve_before_ack_not_fill() {
    let mut p = account();
    let q = quote();
    let id = p.submit(intent("x"), &q, NOW).unwrap();
    assert_eq!(p.orders[&id].state, OrderState::PendingSubmit);
    assert!(p.reserve(None) > d("100"));
    assert!(p.fills.is_empty());
    p.tick(&qs(&q), NOW + 499);
    assert_eq!(p.orders[&id].state, OrderState::PendingSubmit);
    p.tick(&qs(&q), NOW + 500);
    assert_eq!(p.orders[&id].state, OrderState::Accepted);
    assert!(p.fills.is_empty());
}
#[test]
fn duplicate_intent_is_idempotent() {
    let mut p = account();
    let q = quote();
    let i = intent("stable");
    let first = p.submit(i.clone(), &q, NOW).unwrap();
    let reserve = p.reserve(None);
    assert_eq!(first, p.submit(i.clone(), &q, NOW).unwrap());
    assert_eq!(reserve, p.reserve(None));
    let mut changed = i;
    changed.quantity = d("2");
    assert!(p.submit(changed, &q, NOW).is_err());
    assert_eq!(p.orders.len(), 1);
}
#[test]
fn partial_fills_keep_unused_reserve_and_protect_quantity() {
    let mut p = account();
    let mut q = quote();
    q.ask_size = d("5");
    let id = p.submit(intent("x"), &q, NOW).unwrap();
    p.tick(&qs(&q), NOW + 500);
    p.tick(&qs(&q), NOW + 501);
    assert_eq!(p.orders[&id].filled, d("0.5"));
    assert_eq!(p.orders[&id].state, OrderState::PartialFill);
    assert!(p.orders[&id].reserve > d("50"));
    assert_eq!(p.positions[&id].quantity, d("0.5"));
    assert_eq!(p.positions[&id].stop, d("99"));
}
#[test]
fn same_snapshot_liquidity_not_reused() {
    let mut p = account();
    let mut q = quote();
    q.ask_size = d("5");
    p.submit(intent("a"), &q, NOW).unwrap();
    p.submit(intent("b"), &q, NOW).unwrap();
    p.tick(&qs(&q), NOW + 500);
    for t in 501..510 {
        p.tick(&qs(&q), NOW + t);
    }
    assert_eq!(p.fills.iter().map(|f| f.quantity).sum::<D>(), d("0.5"));
}
#[test]
fn old_snapshot_does_not_reset_liquidity() {
    let mut p = account();
    let mut q = quote();
    q.ask_size = d("5");
    p.submit(intent("a"), &q, NOW).unwrap();
    p.tick(&qs(&q), NOW + 500);
    p.tick(&qs(&q), NOW + 501);
    q.ts -= 1;
    p.tick(&qs(&q), NOW + 502);
    assert_eq!(p.fills.len(), 1);
}
#[test]
fn new_snapshot_partial_completion() {
    let mut p = account();
    let mut q = quote();
    q.ask_size = d("5");
    let id = p.submit(intent("x"), &q, NOW).unwrap();
    p.tick(&qs(&q), NOW + 500);
    p.tick(&qs(&q), NOW + 501);
    q.ts += 502;
    q.received_ms += 502;
    p.tick(&qs(&q), NOW + 502);
    assert_eq!(p.orders[&id].state, OrderState::Filled);
    assert_eq!(p.reserve(None), zero());
}
#[test]
fn duplicate_and_out_of_order_fill_rejected() {
    let (mut p, _, id) = entered();
    let f = p.fills[0].clone();
    let cash = p.ledger.cash;
    p.apply_fill(&id, &f.id, f.quantity, f.price, NOW + 999)
        .unwrap();
    assert_eq!(p.ledger.cash, cash);
    assert!(
        p.apply_fill(&id, "another", f.quantity, f.price, NOW + 999)
            .is_err()
    );
    assert_eq!(p.fills.len(), 1);
}
#[test]
fn fill_before_ack_and_overfill_rejected() {
    let mut p = account();
    let q = quote();
    let id = p.submit(intent("x"), &q, NOW).unwrap();
    assert!(p.apply_fill(&id, "bad", d("1"), q.ask, NOW).is_err());
    p.tick(&qs(&q), NOW + 500);
    assert!(p.apply_fill(&id, "bad", d("2"), q.ask, NOW).is_err());
}
#[test]
fn stale_and_future_quotes_block() {
    let p = account();
    let mut q = quote();
    q.ts -= 5001;
    assert!(p.preview(&intent("x"), &q, NOW).is_err());
    q.ts = NOW;
    q.received_ms -= 5001;
    assert!(p.preview(&intent("x"), &q, NOW).is_err());
    q.received_ms = NOW;
    q.ts += 1001;
    assert!(p.preview(&intent("x"), &q, NOW).is_err());
}
#[test]
fn crossed_book_and_currency_block() {
    let p = account();
    let mut q = quote();
    q.bid = d("101");
    assert!(p.preview(&intent("x"), &q, NOW).is_err());
    q = quote();
    q.quote = "USDC".into();
    assert!(p.preview(&intent("x"), &q, NOW).is_err());
}
#[test]
fn missing_stop_or_bad_lot_rejected() {
    let p = account();
    let mut i = intent("x");
    i.stop = None;
    assert!(p.preview(&i, &quote(), NOW).is_err());
    i = intent("x");
    i.quantity = d("0.001");
    assert!(p.preview(&i, &quote(), NOW).is_err());
    i.quantity = d("1.001");
    assert!(p.preview(&i, &quote(), NOW).is_err());
}
#[test]
fn reserve_prevents_double_spending() {
    let mut p = account();
    p.bots.get_mut("manual").unwrap().limits.position = d("1000");
    p.bots.get_mut("manual").unwrap().limits.trade_risk = d("100");
    let q = quote();
    let mut i = intent("first");
    i.quantity = d("9");
    p.submit(i, &q, NOW).unwrap();
    let mut second = intent("second");
    second.quantity = d("2");
    assert!(
        p.submit(second, &q, NOW)
            .unwrap_err()
            .to_string()
            .contains("UNRESERVED")
    );
}
#[test]
fn aggregated_underlying_exposure_across_bots() {
    let mut p = account();
    p.limits.position = d("150");
    let mut b = p.bots["manual"].clone();
    b.id = "second".into();
    b.name = "Second".into();
    b.version = "sma-cross-v1".into();
    b.instrument = "BTC-EUR".into();
    b.instrument = "BTC-EUR".into();
    b.order_eur = d("100");
    b.stop_bps = d("100");
    b.target_bps = d("200");
    p.add_bot(b, NOW).unwrap();
    p.bot_control("second", true, &qs(&quote()), NOW).unwrap();
    p.submit(intent("manual"), &quote(), NOW).unwrap();
    let mut i = intent("bot");
    i.bot_id = "second".into();
    assert!(
        p.submit(i, &quote(), NOW)
            .unwrap_err()
            .to_string()
            .contains("UNDERLYING")
    );
}
#[test]
fn frequency_and_order_count_limits() {
    let mut p = account();
    p.limits.orders_per_minute = 1;
    p.submit(intent("a"), &quote(), NOW).unwrap();
    assert!(
        p.submit(intent("b"), &quote(), NOW)
            .unwrap_err()
            .to_string()
            .contains("FREQUENCY")
    );
    p.limits.orders_per_minute = 6;
    p.limits.orders = 1;
    assert!(
        p.submit(intent("b"), &quote(), NOW)
            .unwrap_err()
            .to_string()
            .contains("ORDER_LIMIT")
    );
}
#[test]
fn spread_and_trade_risk_limits() {
    let mut p = account();
    let mut q = quote();
    q.ask = d("101");
    assert!(
        p.submit(intent("a"), &q, NOW)
            .unwrap_err()
            .to_string()
            .contains("SPREAD")
    );
    p.limits.trade_risk = d("0.1");
    assert!(
        p.submit(intent("b"), &quote(), NOW)
            .unwrap_err()
            .to_string()
            .contains("TRADE_RISK")
    );
}
#[test]
fn limit_touch_does_not_fill() {
    let mut p = account();
    let q = quote();
    let mut i = intent("x");
    i.kind = Kind::Limit;
    i.limit = Some(q.ask);
    let id = p.submit(i, &q, NOW).unwrap();
    p.tick(&qs(&q), NOW + 500);
    p.tick(&qs(&q), NOW + 501);
    assert_eq!(p.orders[&id].state, OrderState::Accepted);
    assert!(p.fills.is_empty());
}
#[test]
fn original_slippage_cap_never_expands() {
    let mut p = account();
    let mut q = quote();
    let id = p.submit(intent("x"), &q, NOW).unwrap();
    let cap = p.orders[&id].cap;
    p.tick(&qs(&q), NOW + 500);
    q.bid = d("100.5");
    q.ask = d("100.6");
    q.ts += 501;
    q.received_ms += 501;
    p.tick(&qs(&q), NOW + 501);
    assert!(p.fills.is_empty());
    assert_eq!(p.orders[&id].cap, cap);
}
#[test]
fn cancel_holds_reserve_until_ack() {
    let mut p = account();
    let q = quote();
    let id = p.submit(intent("x"), &q, NOW).unwrap();
    p.request_cancel(&id, NOW + 1).unwrap();
    assert!(p.reserve(None) > zero());
    p.tick(&Quotes::new(), NOW + 2);
    assert_eq!(p.reserve(None), zero());
    assert_eq!(p.orders[&id].state, OrderState::Canceled);
}
#[test]
fn unknown_holds_reserve_never_retries() {
    let mut p = account();
    let q = quote();
    let id = p.submit(intent("x"), &q, NOW).unwrap();
    p.orders.get_mut(&id).unwrap().state = OrderState::Unknown;
    let reserve = p.reserve(None);
    p.recover(&qs(&q), NOW + 1);
    p.tick(&qs(&q), NOW + 999);
    assert_eq!(p.orders[&id].state, OrderState::Unknown);
    assert_eq!(p.reserve(None), reserve);
    assert!(p.fills.is_empty());
    assert!(p.request_cancel(&id, NOW + 999).is_err());
}
#[test]
fn reserve_and_limits_survive_serialized_restart() {
    let mut p = account();
    p.submit(intent("x"), &quote(), NOW).unwrap();
    p.ledger.high_water = d("10100");
    p.ledger.day_start = d("10050");
    p.ledger.paused = true;
    p.ledger.reason = "USER_PAUSE".into();
    let mut restored: Paper = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    restored.recover(&qs(&quote()), NOW + 10);
    assert_eq!(p.reserve(None), restored.reserve(None));
    assert_eq!(p.ledger.high_water, restored.ledger.high_water);
    assert_eq!(p.ledger.day_start, restored.ledger.day_start);
    assert!(restored.ledger.paused);
}
#[test]
fn equity_includes_unrealized_and_both_fees_no_double_reserve() {
    let (mut p, q, _) = entered();
    assert_eq!(p.ledger.equity, d("9999.6999"));
    let equity = p.ledger.equity;
    p.submit(intent("second"), &q, NOW + 600).unwrap();
    p.mark(&qs(&q), NOW + 600);
    assert_eq!(p.ledger.equity, equity);
}
#[test]
fn account_loss_stop_latched_and_not_reset_by_restart() {
    let (mut p, mut q, _) = entered();
    p.limits.daily_loss = d("1");
    q.bid = d("98");
    q.ask = d("98.1");
    q.ts += 600;
    q.received_ms += 600;
    p.mark(&qs(&q), NOW + 600);
    assert!(p.ledger.paused);
    assert_eq!(p.ledger.reason, "DAY_LOSS");
    let mut restored: Paper = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    restored.recover(&qs(&q), NOW + 601);
    assert!(restored.ledger.paused);
    assert!(
        restored
            .control("RESUME_ENTRIES", &qs(&q), NOW + 601)
            .is_err()
    );
}
#[test]
fn stale_position_freezes_valuation_and_other_entries() {
    let (mut p, _, _) = entered();
    let equity = p.ledger.equity;
    p.mark(&Quotes::new(), NOW + 6000);
    assert!(!p.valuation_fresh);
    assert_eq!(p.ledger.equity, equity);
    let mut q = quote();
    q.ts = NOW + 6000;
    q.received_ms = q.ts;
    assert!(p.preview(&intent("new"), &q, NOW + 6000).is_err());
}
#[test]
fn stop_survives_entry_pause_and_closes_with_inventory_bound() {
    let (mut p, mut q, id) = entered();
    p.control("PAUSE_ENTRIES", &qs(&q), NOW + 550).unwrap();
    q.bid = d("98");
    q.ask = d("98.1");
    q.ts += 600;
    q.received_ms += 600;
    p.tick(&qs(&q), NOW + 600);
    let exit = p.positions[&id].closing_order.clone().unwrap();
    assert!(p.request_cancel(&exit, NOW + 601).is_err());
    p.tick(&qs(&q), NOW + 1100);
    p.tick(&qs(&q), NOW + 1101);
    assert_eq!(p.positions[&id].quantity, zero());
    assert_eq!(p.orders[&exit].state, OrderState::Filled);
    assert!(p.ledger.paused);
    assert_eq!(p.ledger.loss_streak, 1);
}
#[test]
fn partial_entry_rest_canceled_before_stop_exit() {
    let mut p = account();
    let mut q = quote();
    q.ask_size = d("5");
    let id = p.submit(intent("x"), &q, NOW).unwrap();
    p.tick(&qs(&q), NOW + 500);
    p.tick(&qs(&q), NOW + 501);
    q.bid = d("98");
    q.ask = d("98.1");
    q.ts += 600;
    q.received_ms += 600;
    p.tick(&qs(&q), NOW + 600);
    assert_eq!(p.orders[&id].state, OrderState::Canceled);
    assert_eq!(p.orders[&id].reserve, zero());
    let exit = &p.orders[p.positions[&id].closing_order.as_ref().unwrap()];
    assert_eq!(exit.quantity, d("0.5"));
}
#[test]
fn emergency_is_persistent_even_when_loss_reason_changes() {
    let (mut p, mut q, id) = entered();
    p.control("EMERGENCY_FLATTEN", &qs(&q), NOW + 550).unwrap();
    p.limits.daily_loss = d("0.01");
    p.mark(&qs(&q), NOW + 600);
    assert!(p.flatten);
    assert_eq!(p.ledger.reason, "DAY_LOSS");
    q.ts += 601;
    q.received_ms += 601;
    p.tick(&qs(&q), NOW + 601);
    assert!(p.positions[&id].closing_order.is_some());
}
#[test]
fn inventory_cannot_be_sold_twice() {
    let (mut p, q, id) = entered();
    let mut i = intent("sell");
    i.side = Side::Sell;
    i.stop = None;
    i.target = None;
    i.position_id = Some(id);
    p.submit(i.clone(), &q, NOW + 550).unwrap();
    i.client_id = "sell2".into();
    assert!(p.submit(i, &q, NOW + 550).is_err());
}
#[test]
fn daily_boundary_uses_previous_equity_and_keeps_highwater() {
    let mut l = Ledger::new(d("1000"), DAY - 1);
    let limits = Limits::preset(d("1000"));
    l.mark(d("990"), DAY - 1, &limits);
    l.mark(d("985"), DAY, &limits);
    assert_eq!(l.day_start, d("990"));
    assert_eq!(l.daily_loss, d("5"));
    assert_eq!(l.total_loss, d("15"));
    assert_eq!(l.high_water, d("1000"));
}
#[test]
fn all_loss_limits_are_latched() {
    for (field, reason) in [(0, "TOTAL_LOSS"), (1, "DRAWDOWN"), (2, "LOSS_STREAK")] {
        let mut l = Ledger::new(d("1000"), NOW);
        let mut limits = Limits::preset(d("1000"));
        limits.daily_loss = d("1000");
        if field == 0 {
            limits.total_loss = d("5");
        }
        if field == 1 {
            limits.drawdown = d("5");
        }
        if field == 2 {
            l.loss_streak = 3;
        }
        l.mark(d("990"), NOW, &limits);
        assert!(l.paused);
        assert_eq!(l.reason, reason);
    }
}
#[test]
fn time_reversal_pauses_entries() {
    let mut p = account();
    p.tick(&qs(&quote()), NOW - 1);
    assert!(p.ledger.paused);
    assert_eq!(p.ledger.reason, "CLOCK_ROLLBACK");
}
#[test]
fn deterministic_sma_has_warmup_and_no_future_access() {
    assert_eq!(sma_signal(&[d("1"), d("1")], 2, 3), None);
    assert_eq!(
        sma_signal(&[d("3"), d("2"), d("1"), d("3")], 2, 3),
        Some("HOLD")
    );
    assert_eq!(
        sma_signal(&[d("3"), d("2"), d("1"), d("8")], 2, 3),
        Some("BUY")
    );
}

#[test]
fn native_strategy_risk_benchmark() {
    let p = account();
    let q = quote();
    let mut b = p.bots["manual"].clone();
    b.instrument = "BTC-EUR".into();
    b.order_eur = d("100");
    b.stop_bps = d("100");
    b.target_bps = d("200");
    let mut closes = vec![d("100"); 51];
    closes[50] = d("101");
    let mut samples = Vec::new();
    for _ in 0..5000 {
        let start = std::time::Instant::now();
        assert_eq!(
            sma_signal(std::hint::black_box(&closes), 20, 50),
            Some("BUY")
        );
        let i = strategy_intent(&b, std::hint::black_box(&q), NOW);
        std::hint::black_box(p.preview(&i, &q, NOW).unwrap());
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "PHASE2_NATIVE_BENCHMARK {}",
        serde_json::json!({"samples":samples.len(),"p50_ms":samples[2500],"p95_ms":samples[4750],"p99_ms":samples[4950],"scope":"SMA 20/50 + sizing + risk preview; 1 fixture market/manual wallet, no DB/network, 5000 serial samples"})
    );
}

#[test]
fn rejected_exit_does_not_disable_position_protection() {
    let (mut p, q, id) = entered();
    let mut i = intent("exit");
    i.side = Side::Sell;
    i.stop = None;
    i.target = None;
    i.position_id = Some(id.clone());
    let exit = p.submit(i, &q, NOW + 600).unwrap();
    assert_eq!(p.positions[&id].closing_order, Some(exit.clone()));
    p.reject(&exit, NOW + 601, "test rejection");
    assert!(p.positions[&id].closing_order.is_none());
    assert_eq!(p.positions[&id].stop, d("99"));
}
#[test]
fn distinct_position_count_includes_pending_entries() {
    let mut p = account();
    p.limits.positions = 1;
    p.submit(intent("btc"), &quote(), NOW).unwrap();
    let mut q = quote();
    q.instrument = "ETH-EUR".into();
    q.base = "ETH".into();
    let mut i = intent("eth");
    i.instrument = q.instrument.clone();
    assert!(
        p.submit(i, &q, NOW)
            .unwrap_err()
            .to_string()
            .contains("OPEN_POSITION_LIMIT")
    );
}

#[test]
fn candle_exit_is_durable_retries_stale_quotes_and_blocks_entries() {
    let (mut p, mut q, id) = entered();
    p.bots.get_mut("manual").unwrap().running = true;
    p.candle_halt("manual", true, true, NOW + 600).unwrap();
    assert!(!p.bots["manual"].running);
    p = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    p.tick(&qs(&q), NOW + 10000);
    assert!(p.positions[&id].closing_order.is_none());
    assert!(p.bot_exits.contains("manual"));
    q.ts = NOW + 10001;
    q.received_ms = q.ts;
    assert!(
        p.submit(intent("forbidden"), &q, q.ts)
            .unwrap_err()
            .to_string()
            .contains("BOT_EXIT_IN_PROGRESS")
    );
    p.tick(&qs(&q), q.ts);
    assert!(p.positions[&id].closing_order.is_some());
    p.tick(&qs(&q), q.ts + 500);
    p.tick(&qs(&q), q.ts + 501);
    p.tick(&qs(&q), q.ts + 502);
    assert_eq!(p.positions[&id].quantity, zero());
    assert!(!p.bot_exits.contains("manual"));
    assert!(!p.bots["manual"].running);
}
#[test]
fn candle_pause_keeps_position_and_cancels_pending_entry() {
    let (mut p, q, id) = entered();
    p.candle_halt("manual", false, true, NOW + 600).unwrap();
    p.tick(&qs(&q), NOW + 601);
    assert_eq!(p.positions[&id].quantity, D::ONE);
    assert!(!p.bot_exits.contains("manual"));
    let mut p = account();
    let id = p.submit(intent("pending"), &q, NOW).unwrap();
    p.candle_halt("manual", true, true, NOW + 1).unwrap();
    p.tick(&qs(&q), NOW + 600);
    assert_eq!(p.orders[&id].state, OrderState::Canceled);
    assert!(p.fills.is_empty());
}
#[test]
fn candle_close_only_keeps_bot_running_and_old_storage_still_loads() {
    let (mut p, q, id) = entered();
    p.bots.get_mut("manual").unwrap().running = true;
    p.candle_halt("manual", true, false, NOW + 600).unwrap();
    p.tick(&qs(&q), NOW + 601);
    assert!(p.bots["manual"].running);
    assert!(p.positions[&id].closing_order.is_some());
    let mut old = serde_json::to_value(account()).unwrap();
    old.as_object_mut().unwrap().remove("bot_exits");
    let restored: Paper = serde_json::from_value(old).unwrap();
    assert!(restored.bot_exits.is_empty());
}
