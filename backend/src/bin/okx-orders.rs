//! Separate exchange executor. No HTTP endpoint, browser code, Pine or AI.
#[path = "../exchange.rs"]
#[allow(dead_code)]
mod exchange;
#[path = "../order_gateway_ledger.rs"]
mod order_gateway_ledger;
#[path = "../native_bot_risk.rs"]
mod native_bot_risk;
#[path = "../gateway_neuro.rs"]
mod gateway_neuro;
#[path = "../order_ledger.rs"]
mod order_ledger;
#[path = "../order_risk.rs"]
#[allow(dead_code)]
mod order_risk;
#[path = "../order_types.rs"]
#[allow(dead_code)]
mod order_types;
#[path = "../order_wallet.rs"]
#[allow(dead_code)]
mod order_wallet;
#[path = "../secrets.rs"]
#[allow(dead_code)]
mod secrets;
use anyhow::{Result, ensure};
use order_risk::*;
use order_types::*;
use rust_decimal::Decimal as D;
use serde_json::{Value, json};
use sqlx::{
    Connection, Row,
    postgres::{PgConnectOptions, PgConnection, PgPoolOptions},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
    time::Duration,
};
fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
fn dec(v: &Value) -> Result<D> {
    D::from_str(
        v.as_str()
            .ok_or_else(|| anyhow::anyhow!("Onverwacht decimaalveld"))?,
    )
    .map_err(Into::into)
}
fn optional(v: &Value) -> Result<D> {
    if v.is_null() || v == "" {
        Ok(D::ZERO)
    } else {
        dec(v)
    }
}
fn rows(v: &Value) -> Result<&Vec<Value>> {
    v["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Onverwacht OKX-schema"))
}
fn mode() -> Result<ExecutionMode> {
    match std::env::var("EXECUTION_MODE").unwrap_or_default().as_str() {
        "OKX_DEMO" => Ok(ExecutionMode::OkxDemo),
        "LIVE" => Ok(ExecutionMode::Live),
        _ => anyhow::bail!("Gateway vereist OKX_DEMO of LIVE"),
    }
}
fn exchange_mode(mode: ExecutionMode) -> exchange::Mode {
    if mode == ExecutionMode::OkxDemo {
        exchange::Mode::OkxDemo
    } else {
        exchange::Mode::Live
    }
}
async fn status(
    db: &sqlx::PgPool,
    mode: ExecutionMode,
    state: &str,
    reason: &str,
    snapshot: Option<&AccountSnapshot>,
) -> Result<()> {
    sqlx::query("UPDATE execution.accounts SET status=$2,reason=$3,updated_ms=$4,snapshot=COALESCE($5,snapshot) WHERE mode=$1").bind(mode.id()).bind(state).bind(reason).bind(now()).bind(snapshot.map(serde_json::to_value).transpose()?).execute(db).await?;
    Ok(())
}
async fn wallet(okx: &exchange::Okx, db: &sqlx::PgPool, mode: ExecutionMode) -> Result<()> {
    let config = okx.get("/api/v5/account/config").await?;
    let c = rows(&config)?
        .first()
        .ok_or_else(|| anyhow::anyhow!("Accountconfiguratie ontbreekt"))?;
    let uid = c["uid"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_alphanumeric()))
        .ok_or_else(|| anyhow::anyhow!("Accountidentiteit ontbreekt"))?
        .to_string();
    let permission = c["perm"]
        .as_str()
        .unwrap_or("")
        .split(',')
        .any(|p| p.trim() == "trade");
    let trading = okx.get("/api/v5/account/balance").await?;
    let trading_received_ms = now();
    let details = rows(&trading)?
        .first()
        .and_then(|v| v["details"].as_array())
        .ok_or_else(|| anyhow::anyhow!("Handelssaldo ontbreekt"))?;
    let trading = order_wallet::assets(details, false)?;
    let (funding, funding_reason) = match okx
        .get("/api/v5/asset/balances")
        .await
        .and_then(|v| order_wallet::assets(rows(&v)?, true))
    {
        Ok(a) => (Some(a), String::new()),
        Err(_) => (
            None,
            "Funding-saldo kon niet worden opgehaald; handelssaldo is apart gecontroleerd.".into(),
        ),
    };
    let w = order_wallet::Wallet {
        uid,
        mode,
        received_ms: now(),
        trading_received_ms,
        trade_permission: permission,
        trading,
        funding,
        funding_reason,
    };
    sqlx::query("UPDATE execution.wallets SET snapshot=$2,status='BALANCES_CONNECTED',reason='Actueel OKX-saldo',updated_ms=$3 WHERE mode=$1").bind(mode.id()).bind(serde_json::to_value(w)?).bind(now()).execute(db).await?;
    Ok(())
}
async fn release_snapshot(db: &sqlx::PgPool, s: &mut AccountSnapshot) -> Result<()> {
    let row = sqlx::query(
        "SELECT policy,policy_revision,live_release FROM execution.accounts WHERE mode=$1",
    )
    .bind(s.mode.id())
    .fetch_one(db)
    .await?;
    let p = row
        .get::<Option<Value>, _>("policy")
        .map(serde_json::from_value::<TradingLimits>)
        .transpose()?;
    let release = row
        .get::<Option<Value>, _>("live_release")
        .map(serde_json::from_value::<LiveRelease>)
        .transpose()?;
    apply_live_release(
        s,
        p.as_ref(),
        release.as_ref(),
        row.get("policy_revision"),
        now(),
    );
    Ok(())
}
async fn account(
    okx: &exchange::Okx,
    db: &sqlx::PgPool,
    mode: ExecutionMode,
) -> Result<AccountSnapshot> {
    let time = okx.get("/api/v5/public/time").await?;
    ensure!(
        (now()
            - rows(&time)?[0]["ts"]
                .as_str()
                .unwrap_or("0")
                .parse::<i64>()?)
        .abs()
            < 1000,
        "Klokafwijking te groot"
    );
    let config = okx.get("/api/v5/account/config").await?;
    let config = rows(&config)?
        .first()
        .ok_or_else(|| anyhow::anyhow!("Accountconfiguratie ontbreekt"))?;
    let uid = config["uid"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Account-id ontbreekt"))?
        .to_string();
    let permission = config["perm"]
        .as_str()
        .unwrap_or("")
        .split(',')
        .any(|s| s.trim() == "trade");
    let meta = okx.get("/api/v5/account/instruments?instType=SPOT").await?;
    let balance = okx.get("/api/v5/account/balance").await?;
    let detail = rows(&balance)?
        .first()
        .and_then(|v| v["details"].as_array())
        .ok_or_else(|| anyhow::anyhow!("Rekeningsaldo ontbreekt"))?;
    let mut available: BTreeMap<String, D> = BTreeMap::new();
    let mut holdings: BTreeMap<String, D> = BTreeMap::new();
    for item in detail {
        let ccy = item["ccy"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Saldovaluta ontbreekt"))?;
        let total = optional(&item["cashBal"])?;
        let free = optional(&item["availBal"])?;
        ensure!(
            free >= D::ZERO && total >= D::ZERO && optional(&item["liab"])? == D::ZERO,
            "Geen short- of geleende spotinventaris toegestaan"
        );
        holdings.insert(ccy.into(), total);
        available.insert(ccy.into(), free);
    }
    // Account instruments determine eligibility. Refresh execution books/fees only
    // for selected or outstanding instruments; selecting hundreds of allowed pairs
    // must not turn into hundreds of private calls every reconciliation cycle.
    let selected:Vec<String>=sqlx::query_scalar("SELECT instrument FROM execution.market_interest WHERE mode=$1 AND updated_ms>$2 ORDER BY updated_ms DESC LIMIT 4").bind(mode.id()).bind(now()-60000).fetch_all(db).await?;
    let active:Vec<String>=sqlx::query_scalar("SELECT DISTINCT ticket->>'instrument' FROM execution.orders WHERE mode=$1 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).fetch_all(db).await?;
    let mut wanted:BTreeSet<String>=selected.into_iter().chain(active).collect();
    wanted.insert("BTC-EUR".into());
    let prices=okx.get("/api/v5/market/tickers?instType=SPOT").await?;
    let mut quotes=BTreeMap::new();
    for instrument in rows(&meta)?.iter().filter(|i|i["state"]=="live") {
        let Some(id)=instrument["instId"].as_str() else {continue;};
        if !id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'){continue;}
        let Some(t)=rows(&prices)?.iter().find(|t|t["instId"]==id) else {continue;};
        let (Ok(bid),Ok(ask))=(dec(&t["bidPx"]),dec(&t["askPx"])) else {continue;};
        if bid<=D::ZERO||ask<bid{continue;}
        quotes.insert(id.into(),ExecutionQuote{bid,ask,received_ms:now(),rules:MarketRules{
            tick:dec(&instrument["tickSz"])?,lot:dec(&instrument["lotSz"])?,minimum:dec(&instrument["minSz"])?,base:instrument["baseCcy"].as_str().unwrap_or("").into(),quote:instrument["quoteCcy"].as_str().unwrap_or("").into()}});
    }
    let mut fee_rate=D::ZERO;
    let mut fee_rates=BTreeMap::new();
    for id in wanted {
        let Some(q)=quotes.get_mut(&id) else {continue;};
        let book=okx.get(&format!("/api/v5/market/books?instId={id}&sz=5")).await?;
        let b=rows(&book)?.first().ok_or_else(||anyhow::anyhow!("Orderboek ontbreekt"))?;
        q.bid=dec(&b["bids"][0][0])?;q.ask=dec(&b["asks"][0][0])?;q.received_ms=now();
        let fees=okx.get(&format!("/api/v5/account/trade-fee?instType=SPOT&instId={id}")).await?;
        let f=rows(&fees)?.first().ok_or_else(||anyhow::anyhow!("Marktfee ontbreekt"))?;
        let fee=dec(&f["taker"])?.abs().max(dec(&f["maker"])?.abs());
        fee_rate=fee_rate.max(fee);fee_rates.insert(id,fee);
    }
    ensure!(!fee_rates.is_empty(),"Geen accountbeschikbare spotmarkt met gecontroleerde fee");
    let mut complete=true;let mut equity=D::ZERO;
    // Valuation reuses the same checked conversion graph as the risk core.
    let valuation=AccountSnapshot{uid:uid.clone(),mode,received_ms:now(),connected:false,reconciled:false,trade_permission:permission,valuation_complete:false,equity_eur:D::ZERO,available:available.clone(),holdings:holdings.clone(),quotes:quotes.clone(),fee_rate,fee_rates:fee_rates.clone(),tradable:vec![],daily_loss:D::ZERO,total_loss:D::ZERO,drawdown:D::ZERO,native_protection_verified:false,live_acceptance_complete:false,live_enabled:false};
    for (ccy,qty) in &holdings {
        if *qty>D::ZERO {
            match euro_bounds(&valuation,ccy,now()) {Ok((bid,_))=>equity+=*qty*bid,Err(_)=>complete=false}
        }
    }
    let pending = okx
        .get("/api/v5/trade/orders-pending?instType=SPOT&limit=100")
        .await?;
    ensure!(
        rows(&pending)?.len() < 100,
        "Open-orderlijst moet eerst volledig gepagineerd worden"
    );
    let known: BTreeSet<String> =
        sqlx::query_scalar("SELECT client_id FROM execution.orders WHERE mode=$1")
            .bind(mode.id())
            .fetch_all(db)
            .await?
            .into_iter()
            .collect();
    let protect_ids: Vec<String> =
        sqlx::query_scalar("SELECT client_id FROM execution.protection WHERE mode=$1")
            .bind(mode.id())
            .fetch_all(db)
            .await?;
    let mut known = known;
    known.extend(protect_ids);
    let children: BTreeSet<String> =
        sqlx::query_scalar("SELECT exchange_id FROM execution.children WHERE mode=$1")
            .bind(mode.id())
            .fetch_all(db)
            .await?
            .into_iter()
            .collect();
    let ordinary_known = rows(&pending)?.iter().all(|r| {
        r["clOrdId"].as_str().is_some_and(|id| known.contains(id))
            || r["ordId"].as_str().is_some_and(|id| children.contains(id))
    });
    let mut algo_known = true;
    for kind in [
        "conditional",
        "oco",
        "trigger",
        "move_order_stop",
        "smart_iceberg",
        "twap",
    ] {
        let pending = okx
            .get(&format!(
                "/api/v5/trade/orders-algo-pending?ordType={kind}&instType=SPOT&limit=100"
            ))
            .await?;
        ensure!(
            rows(&pending)?.len() < 100,
            "Algo-orderlijst moet eerst volledig gepagineerd worden"
        );
        algo_known &= rows(&pending)?.iter().all(|r| {
            r["algoClOrdId"]
                .as_str()
                .is_some_and(|id| known.contains(id))
        });
    }
    // Accounting memory is never reset by restart or an ordinary policy change.
    let mut tx = db.begin().await?;
    let memory: Value =
        sqlx::query_scalar("SELECT risk_memory FROM execution.accounts WHERE mode=$1 FOR UPDATE")
            .bind(mode.id())
            .fetch_one(&mut *tx)
            .await?;
    ensure!(
        memory["uid"].as_str().is_none_or(|old| old == uid),
        "Accountidentiteit gewijzigd; risicoadministratie blijft bewaard"
    );
    let anchored = memory["uid"].is_string();
    let mut flow = D::ZERO;
    let mut unexplained = false;
    let bills=sqlx::query("SELECT bill_id,currency,balance_change::text,kind,detail FROM execution.bills WHERE mode=$1 AND risk_applied=false FOR UPDATE").bind(mode.id()).fetch_all(&mut *tx).await?;
    if anchored {
        for b in &bills {
            let kind: String = b.get("kind");
            let change: D = b.get::<String, _>("balance_change").parse()?;
            if kind == "1" {
                if b.get::<String, _>("currency") == "EUR" {
                    flow += change;
                } else if change != D::ZERO {
                    unexplained = true;
                }
            } else if kind == "2" {
                let d: Value = b.get("detail");
                let external = d["ordId"].as_str().unwrap_or("");
                if !children.contains(external)
                    && !d["clOrdId"].as_str().is_some_and(|id| known.contains(id))
                {
                    unexplained = true;
                }
            } else if change != D::ZERO {
                unexplained = true;
            }
        }
    }
    let number = |key: &str, default: D| {
        memory[key]
            .as_str()
            .and_then(|s| s.parse::<D>().ok())
            .unwrap_or(default)
    };
    let start = number("start", equity) + flow;
    let high = (number("high", equity) + flow).max(equity);
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let day_start = if memory["day"] == day {
        number("day_start", equity) + flow
    } else {
        equity
    };
    let unexplained = unexplained || memory["unexplained"] == true;
    sqlx::query("UPDATE execution.accounts SET risk_memory=$2 WHERE mode=$1").bind(mode.id()).bind(json!({"uid":uid,"start":start.to_string(),"high":high.to_string(),"day":day,"day_start":day_start.to_string(),"unexplained":unexplained,"cashflow_acceptance":false})).execute(&mut *tx).await?;
    sqlx::query("UPDATE execution.bills SET risk_applied=true WHERE mode=$1")
        .bind(mode.id())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    complete &= !unexplained;
    let tradable = rows(&meta)?
        .iter()
        .filter(|r| r["state"] == "live" && (r["quoteCcy"] == "EUR" || r["quoteCcy"] == "USDC"))
        .filter_map(|r| r["instId"].as_str().map(String::from))
        .collect();
    Ok(AccountSnapshot {
        uid,
        mode,
        tradable,
        received_ms: now(),
        connected: true,
        reconciled: ordinary_known && algo_known,
        trade_permission: permission,
        valuation_complete: complete,
        equity_eur: equity,
        available,
        holdings,
        quotes,
        fee_rate,
        fee_rates,
        daily_loss: (day_start - equity).max(D::ZERO),
        total_loss: (start - equity).max(D::ZERO),
        drawdown: (high - equity).max(D::ZERO),
        native_protection_verified: false,
        live_acceptance_complete: false,
        live_enabled: false,
    })
}
async fn event(
    db: &sqlx::PgPool,
    mode: ExecutionMode,
    id: &str,
    state: &str,
    detail: &Value,
) -> Result<()> {
    sqlx::query("INSERT INTO execution.events(mode,client_id,kind,detail,created_ms) VALUES($1,$2,$3,$4,$5)").bind(mode.id()).bind(id).bind(state).bind(detail).bind(now()).execute(db).await?;
    Ok(())
}
async fn set_order(
    db: &sqlx::PgPool,
    mode: ExecutionMode,
    id: &str,
    state: &str,
    external: Option<&str>,
    filled: D,
    detail: &Value,
    reason: &str,
) -> Result<()> {
    let final_state = ["FILLED", "CANCELED", "REJECTED"].contains(&state);
    sqlx::query("UPDATE execution.orders SET state=$3,exchange_id=COALESCE($4,exchange_id),filled=GREATEST(filled,$5::numeric),result=$6,reason=$7,updated_ms=$8,reserve=CASE WHEN $9 THEN 0 ELSE reserve END WHERE mode=$1 AND client_id=$2 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).bind(id).bind(state).bind(external).bind(filled.to_string()).bind(detail).bind(reason.chars().take(500).collect::<String>()).bind(now()).bind(final_state).execute(db).await?;
    event(db, mode, id, state, detail).await?;
    Ok(())
}
async fn reconcile(okx: &exchange::Okx, db: &sqlx::PgPool, mode: ExecutionMode) -> Result<()> {
    let orders=sqlx::query("SELECT client_id,ticket,compiled,exchange_id,state,command,reduction_parent_id FROM execution.orders WHERE mode=$1 AND state NOT IN ('QUEUED','FILLED','CANCELED','REJECTED') ORDER BY created_ms LIMIT 100").bind(mode.id()).fetch_all(db).await?;
    for row in orders {
        let id: String = row.get("client_id");
        let ticket: Ticket = serde_json::from_value(row.get("ticket"))?;
        let c: CompiledOrder = serde_json::from_value(row.get("compiled"))?;
        let path = if c.algo {
            format!("/api/v5/trade/order-algo?algoClOrdId={id}")
        } else {
            format!(
                "/api/v5/trade/order?instId={}&clOrdId={id}",
                ticket.instrument
            )
        };
        let result = match okx.get(&path).await {
            Ok(v) => v,
            Err(_) => {
                set_order(
                    db,
                    mode,
                    &id,
                    "UNKNOWN",
                    None,
                    D::ZERO,
                    &json!({"unresolved":true}),
                    "Status nog niet verklaard; geen herverzending",
                )
                .await?;
                continue;
            }
        };
        let data = rows(&result)?
            .first()
            .ok_or_else(|| anyhow::anyhow!("Orderdetail ontbreekt"))?;
        let exchange_id = data[if c.algo { "algoId" } else { "ordId" }]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Exchange-identiteit ontbreekt"))?;
        let mut filled = if c.algo {
            D::ZERO
        } else {
            optional(&data["accFillSz"])?
        };
        if !c.algo && filled > D::ZERO {
            order_gateway_ledger::fills(okx, db, mode, row.get::<Option<String>,_>("reduction_parent_id").as_deref().unwrap_or(&id), exchange_id, filled).await?;
        }
        let mut children_terminal = true;
        let mut has_children = false;
        if c.algo {
            if let Some(children) = data["ordIdList"].as_array() {
                for child in children {
                    let child = child
                        .as_str()
                        .filter(|s| s.bytes().all(|b| b.is_ascii_alphanumeric()))
                        .ok_or_else(|| anyhow::anyhow!("Ongeldig child-order-id"))?;
                    sqlx::query("INSERT INTO execution.children(mode,parent_id,exchange_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(mode.id()).bind(&id).bind(child).execute(db).await?;
                    let d = okx
                        .get(&format!(
                            "/api/v5/trade/order?instId={}&ordId={child}",
                            ticket.instrument
                        ))
                        .await?;
                    let d = rows(&d)?
                        .first()
                        .ok_or_else(|| anyhow::anyhow!("Child-order ontbreekt"))?;
                    let cf = optional(&d["accFillSz"])?;
                    if cf > D::ZERO {
                        order_gateway_ledger::fills(okx, db, mode, &id, child, cf).await?;
                    }
                    filled += cf;
                    children_terminal &= ["filled", "canceled"].iter().any(|s| d["state"] == *s);
                    has_children = true;
                }
            }
        }
        let state = if c.algo {
            order_ledger::algorithm_state(
                data,
                ticket.quantity,
                filled,
                children_terminal,
                has_children,
            )
        } else {
            order_ledger::regular_state(data)?
        };
        ensure!(
            filled >= D::ZERO && filled <= ticket.quantity,
            "Onverklaarde cumulatieve fill"
        );
        if row.get::<String,_>("state")=="UNKNOWN" && state!="UNKNOWN" {event(db,mode,&id,"RECOVERED_UNKNOWN",&json!({"state":state,"exchange_id":exchange_id,"source":"OKX query"})).await?;}
        set_order(db, mode, &id, state, Some(exchange_id), filled, &result, "").await?;
        if row.get::<String, _>("command") == "CANCEL"
            && !["FILLED", "CANCELED", "REJECTED"].contains(&state)
        {
            match okx
                .cancel_native(&ticket.instrument, exchange_id, c.algo)
                .await
            {
                Ok(v) => {
                    set_order(
                        db,
                        mode,
                        &id,
                        "CANCEL_PENDING",
                        Some(exchange_id),
                        filled,
                        &v,
                        "Annulering ontvangen; wacht op definitieve status",
                    )
                    .await?;
                    sqlx::query(
                        "UPDATE execution.orders SET command='NONE' WHERE mode=$1 AND client_id=$2",
                    )
                    .bind(mode.id())
                    .bind(&id)
                    .execute(db)
                    .await?;
                }
                Err(_) => {
                    set_order(
                        db,
                        mode,
                        &id,
                        "UNKNOWN",
                        Some(exchange_id),
                        filled,
                        &json!({"cancel_outcome_unknown":true}),
                        "Annuleringsuitkomst onzeker; status eerst opvragen",
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}
async fn dispatch(
    okx: &exchange::Okx,
    db: &sqlx::PgPool,
    owner: &mut PgConnection,
    mode: ExecutionMode,
    snapshot: &AccountSnapshot,
) -> Result<()> {
    let mut tx = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(if mode == ExecutionMode::OkxDemo {
            478007_i64
        } else {
            478008_i64
        })
        .execute(&mut *tx)
        .await?;
    let row=sqlx::query("SELECT client_id,ticket,compiled,created_ms,bot_release_id,reduction_parent_id FROM execution.orders WHERE mode=$1 AND state='QUEUED' ORDER BY created_ms LIMIT 1 FOR UPDATE").bind(mode.id()).fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        return Ok(());
    };
    let id: String = row.get("client_id");
    let ticket: Ticket = serde_json::from_value(row.get("ticket"))?;
    let account_row=sqlx::query("SELECT policy,policy_revision,live_release FROM execution.accounts WHERE mode=$1 FOR UPDATE").bind(mode.id()).fetch_one(&mut *tx).await?;
    let policy: TradingLimits = serde_json::from_value(
        account_row
            .get::<Option<Value>, _>("policy")
            .ok_or_else(|| anyhow::anyhow!("Bevestigde limieten ontbreken"))?,
    )?;
    let release = account_row
        .get::<Option<Value>, _>("live_release")
        .map(serde_json::from_value::<LiveRelease>)
        .transpose()?;
    let claims=sqlx::query("SELECT client_id,ticket,quantity::text,filled::text,reserve::text,created_ms,state FROM execution.orders WHERE mode=$1 AND (state NOT IN ('FILLED','CANCELED','REJECTED') OR created_ms>$2)").bind(mode.id()).bind(now()-60000).fetch_all(&mut *tx).await?;
    let mut claims = claims
        .iter()
        .map(|r| -> Result<Claim> {
            let t: Ticket = serde_json::from_value(r.get("ticket"))?;
            Ok(Claim {
                id: r.get("client_id"),
                instrument: t.instrument,
                side: t.side,
                quantity: r.get::<String, _>("quantity").parse::<D>()?
                    - r.get::<String, _>("filled").parse::<D>()?,
                reserve: r.get::<String, _>("reserve").parse()?,
                created_ms: r.get("created_ms"),
                active: !["FILLED", "CANCELED", "REJECTED"]
                    .contains(&r.get::<String, _>("state").as_str()),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let protective=sqlx::query("SELECT p.client_id,p.quantity::text,p.created_ms,o.ticket FROM execution.protection p JOIN execution.orders o ON p.mode=o.mode AND p.parent_id=o.client_id WHERE p.mode=$1 AND p.state NOT IN ('FILLED','CANCELED')").bind(mode.id()).fetch_all(&mut *tx).await?;
    for r in protective {
        let t: Ticket = serde_json::from_value(r.get("ticket"))?;
        let q = r.get::<String, _>("quantity").parse::<D>()?;
        claims.push(Claim {
            id: r.get("client_id"),
            instrument: t.instrument,
            side: Direction::Sell,
            quantity: q,
            reserve: q,
            created_ms: r.get("created_ms"),
            active: true,
        });
    }
    let mut snapshot = snapshot.clone();
    apply_live_release(
        &mut snapshot,
        Some(&policy),
        release.as_ref(),
        account_row.get("policy_revision"),
        now(),
    );
    let book = okx
        .get(&format!(
            "/api/v5/market/books?instId={}&sz=5",
            ticket.instrument
        ))
        .await?;
    let b = rows(&book)?
        .first()
        .ok_or_else(|| anyhow::anyhow!("Actueel orderboek ontbreekt"))?;
    let q = snapshot
        .quotes
        .get_mut(&ticket.instrument)
        .ok_or_else(|| anyhow::anyhow!("Accountmarkt ontbreekt"))?;
    q.bid = dec(&b["bids"][0][0])?;
    q.ask = dec(&b["asks"][0][0])?;
    q.received_ms = now();
    let bot_limits=if let Some(rid)=row.get::<Option<String>,_>("bot_release_id") {
        let r=sqlx::query("SELECT to_jsonb(r) AS release,n.status,n.report,n.updated_ms FROM training.releases r JOIN training.native_runs n ON n.release_id=r.id WHERE r.id=$1 FOR UPDATE OF r,n").bind(&rid).fetch_one(&mut *tx).await?;
        let reducing=row.get::<Option<String>,_>("reduction_parent_id").is_some();
        match native_bot_risk::check_binding(&r.get::<Value,_>("release"),&ticket,account_row.get("policy_revision"),&policy,r.get::<String,_>("status")=="RUNNING",reducing) {
            Ok(p)=>{snapshot.live_enabled=mode==ExecutionMode::Live;
                if !reducing {ensure!((0..=5000).contains(&(now()-r.get::<i64,_>("updated_ms"))),"Botrisicostaat is verouderd");let own=order_risk::scoped_bot_snapshot(&snapshot,&r.get::<Value,_>("report"))?;ensure!(own.daily_loss<p.daily_loss&&own.total_loss<p.total_loss&&own.drawdown<p.drawdown,"Botverlieslimiet bereikt");}
                Some(p)},Err(e)=>{
                sqlx::query("UPDATE execution.orders SET state='REJECTED',reserve=0,reason=$3,updated_ms=$4 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).bind(e.to_string()).bind(now()).execute(&mut *tx).await?;tx.commit().await?;return Ok(());
            }
        }
    }else{None};
    if let Some(ref bot)=bot_limits {if ticket.side==Direction::Buy {if let Err(e)=authorize(&ticket,bot,&{
        let rid=row.get::<String,_>("bot_release_id");let report:Value=sqlx::query_scalar("SELECT report FROM training.native_runs WHERE release_id=$1").bind(&rid).fetch_one(&mut *tx).await?;
        order_risk::scoped_bot_snapshot(&snapshot,&report)?
    },&claims.iter().filter(|c|{
        // The shared account pass below includes all claims. The smaller bot cap
        // must not count unrelated manual positions as owned bot inventory.
        c.id==id
    }).cloned().collect::<Vec<_>>(),now(),&id){sqlx::query("UPDATE execution.orders SET state='REJECTED',reserve=0,reason=$3,updated_ms=$4 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).bind(e.to_string()).bind(now()).execute(&mut *tx).await?;tx.commit().await?;return Ok(());}}}
    if ticket.side==Direction::Buy {if let Some(rid)=row.get::<Option<String>,_>("bot_release_id") {
        let release:Value=sqlx::query_scalar("SELECT to_jsonb(r) FROM training.releases r WHERE id=$1").bind(&rid).fetch_one(&mut *tx).await?;
        if let Err(e)=native_bot_risk::check_cost_envelope(&release,&snapshot){sqlx::query("UPDATE execution.orders SET state='REJECTED',reserve=0,reason=$3 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).bind(e.to_string()).execute(&mut *tx).await?;tx.commit().await?;return Ok(());}
    }}
    let authorization=if let Some(parent)=row.get::<Option<String>,_>("reduction_parent_id") {
        let p=sqlx::query("SELECT account_uid,bot_release_id,ticket FROM execution.orders WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&parent).fetch_one(&mut *tx).await?;
        ensure!(p.get::<Option<String>,_>("bot_release_id")==row.get::<Option<String>,_>("bot_release_id"),"Afsluiting hoort niet bij deze bot");
        let original:Ticket=serde_json::from_value(p.get("ticket"))?;ensure!(original.instrument==ticket.instrument&&original.side==Direction::Buy,"Afsluitmarkt of parent gewijzigd");
        let base=ticket.instrument.split('-').next().unwrap();let owned=gateway_neuro::owned_quantity(db,mode,&parent,base).await?;
        authorize_owned_reduction(&ticket,&p.get::<String,_>("account_uid"),&snapshot,&claims,owned,now(),&id)
    }else{authorize(&ticket,&policy,&snapshot,&claims,now(),&id)};
    let auth = match authorization {
        Ok(v) => v,
        Err(e) => {
            sqlx::query("UPDATE execution.orders SET state='REJECTED',reason=$3,reserve=0,updated_ms=$4 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).bind(e.to_string()).bind(now()).execute(&mut *tx).await?;
            tx.commit().await?;
            return Ok(());
        }
    };
    let original: CompiledOrder = serde_json::from_value(row.get("compiled"))?;
    if now() - row.get::<i64, _>("created_ms") > 15000
        || (ticket.side == Direction::Buy && auth.compiled().worst_price > original.worst_price)
    {
        sqlx::query("UPDATE execution.orders SET state='REJECTED',reason='Opdracht verlopen of prijsgrens verslechterd; maak een nieuwe preview',reserve=0,updated_ms=$3 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).bind(now()).execute(&mut *tx).await?;
        tx.commit().await?;
        return Ok(());
    }
    // Persist uncertainty BEFORE network I/O. A process death never recreates QUEUED.
    sqlx::query("UPDATE execution.orders SET state='SUBMITTING',updated_ms=$3 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).bind(now()).execute(&mut *tx).await?;
    tx.commit().await?;
    sqlx::query("SELECT 1").execute(&mut *owner).await?;
    match okx.send_authorized(&auth).await {
        Ok(v) => {
            let external = v["data"][0][if auth.compiled().algo {
                "algoId"
            } else {
                "ordId"
            }]
            .as_str()
            .unwrap();
            set_order(db, mode, &id, "ACCEPTED", Some(external), D::ZERO, &v, "").await?;
        }
        Err(e) => {
            let error = e.to_string();
            let rejected = error.starts_with("OKX_REJECTED:");
            set_order(
                db,
                mode,
                &id,
                if rejected { "REJECTED" } else { "UNKNOWN" },
                None,
                D::ZERO,
                &json!({"known_rejection":rejected}),
                if rejected {
                    &error
                } else {
                    "Verzenduitkomst onzeker; eerst reconciliëren, nooit blind opnieuw sturen"
                },
            )
            .await?;
        }
    }
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("TLS-provider"))?;
    let mode = mode()?;
    // Reading a Live wallet is independent of the personal execution release.
    let password = tokio::fs::read_to_string(
        std::env::var("DATABASE_PASSWORD_FILE")
            .unwrap_or_else(|_| "/run/secrets/db_password".into()),
    )
    .await?;
    let options = PgConnectOptions::new()
        .host(&std::env::var("DATABASE_HOST").unwrap_or_else(|_| "db".into()))
        .username("terminal")
        .database(&std::env::var("DATABASE_NAME").unwrap_or_else(|_| "terminal".into()))
        .password(password.trim());
    let db = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(options.clone())
        .await?;
    let mut owner = PgConnection::connect_with(&options).await?;
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(if mode == ExecutionMode::OkxDemo {
            478107_i64
        } else {
            478108_i64
        })
        .fetch_one(&mut owner)
        .await?;
    ensure!(locked, "Er draait al een ordergateway voor deze omgeving");
    event(&db,mode,"","GATEWAY_STARTED",&json!({"code_version":native_bot_risk::code_version()})).await?;
    sqlx::query("UPDATE execution.orders SET state='UNKNOWN',reason='Herstart tijdens verzending; eerst reconciliëren' WHERE mode=$1 AND state='SUBMITTING'").bind(mode.id()).execute(&db).await?;
    let purpose = if mode == ExecutionMode::Live {
        "okx_live"
    } else {
        "okx_demo"
    };
    let (adapter, key_digest) = loop {
        match secrets::read(purpose).await {
            Ok(v) => {
                let d = ring::digest::digest(&ring::digest::SHA256, v.as_bytes())
                    .as_ref()
                    .to_vec();
                let credentials = serde_json::from_str::<exchange::Credentials>(&v)?;
                break (
                    std::sync::Arc::new(exchange::Okx::new(exchange_mode(mode), credentials)?),
                    d,
                );
            }
            Err(_) => {
                status(
                    &db,
                    mode,
                    "KEY_REQUIRED",
                    "Stel je OKX-aansluiting in onder Systeem → Aansluitingen & AI.",
                    None,
                )
                .await?;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    };
    let private = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let business = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let wake = std::sync::Arc::new(tokio::sync::Notify::new());
    for (is_business, health) in [(false, private.clone()), (true, business.clone())] {
        let adapter = adapter.clone();
        let wake = wake.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e)=adapter.stream(is_business, &health, &wake).await {eprintln!("{} {} verbinding: {}",mode.id(),if is_business{"business"}else{"private"},e.to_string().chars().take(180).collect::<String>());}
                health.store(false, std::sync::atomic::Ordering::SeqCst);
                wake.notify_one();
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }
    loop {
        let current = secrets::read(purpose).await.ok().map(|v| {
            ring::digest::digest(&ring::digest::SHA256, v.as_bytes())
                .as_ref()
                .to_vec()
        });
        if current.as_ref() != Some(&key_digest) {
            sqlx::query("UPDATE execution.accounts SET live_release=NULL WHERE mode=$1")
                .bind(mode.id())
                .execute(&db)
                .await?;
            sqlx::query("UPDATE execution.wallets SET status='KEY_CHANGED',reason='Aansluiting gewijzigd; opnieuw verbinden',updated_ms=$2 WHERE mode=$1").bind(mode.id()).bind(now()).execute(&db).await?;
            sqlx::query("UPDATE execution.accounts SET snapshot=jsonb_set(COALESCE(snapshot,'{}'),'{connected}','false') WHERE mode=$1").bind(mode.id()).execute(&db).await?;
            status(
                &db,
                mode,
                "KEY_CHANGED",
                "Key gewijzigd; gateway herstart en synchroniseert opnieuw.",
                None,
            )
            .await?;
            return Ok(());
        }
        // No POST, order, transfer or permission to trade is required for this read.
        if let Err(e) = wallet(&adapter, &db, mode).await {
            sqlx::query("UPDATE execution.wallets SET status='CONNECTION_ERROR',reason=$2,updated_ms=$3 WHERE mode=$1").bind(mode.id()).bind(format!("OKX-saldo niet bereikbaar: {}",e.to_string().chars().take(200).collect::<String>())).bind(now()).execute(&db).await?;
        }
        match async {
            sqlx::query("SELECT 1").execute(&mut owner).await?;
            reconcile(&adapter,&db,mode).await?;
            if private.load(std::sync::atomic::Ordering::SeqCst)&&business.load(std::sync::atomic::Ordering::SeqCst){gateway_neuro::deadlines(&db,mode).await?;}
            order_gateway_ledger::protective_orders(&adapter,&db,mode).await?;
            let protected=order_gateway_ledger::protect(&adapter,&db,mode).await?;
            order_gateway_ledger::bills(&adapter,&db,mode).await?;
            let mut snapshot=account(&adapter,&db,mode).await?;
            let feeds=private.load(std::sync::atomic::Ordering::SeqCst)&&business.load(std::sync::atomic::Ordering::SeqCst);
            let unresolved:i64=sqlx::query_scalar("SELECT count(*) FROM execution.orders WHERE mode=$1 AND state IN ('UNKNOWN','SUBMITTING')").bind(mode.id()).fetch_one(&db).await?;
            snapshot.connected=feeds;snapshot.reconciled&=unresolved==0&&feeds;snapshot.native_protection_verified=protected&&feeds;
            release_snapshot(&db,&mut snapshot).await?;
            gateway_neuro::close_positions(&db,mode,&snapshot).await?;
            let reason=if !feeds{"Private verbinding niet volledig bevestigd; nieuwe orders wachten."}else if !snapshot.reconciled{"Account nog niet volledig gesynchroniseerd; controleer externe of onzekere orders."}else if !snapshot.trade_permission{"Saldo verbonden; deze API-key heeft geen handelsrechten."}else if !protected{"Bescherming nog niet bevestigd of fill onder minimum; nieuwe aankopen geblokkeerd."}else if mode==ExecutionMode::Live && snapshot.live_enabled{"Live verbonden en vrijgegeven voor je bevestigde account en limieten."}else if mode==ExecutionMode::Live{"Live-account verbonden. Bevestig je handelslimieten en geef Live vrij."}else{"OKX Demo-account verbonden. Bevestig je account en handelslimieten."};
            status(&db,mode,if snapshot.live_enabled{"LIVE_ENABLED"}else if snapshot.reconciled{"ACCOUNT_CONNECTED"}else{"RECONCILING"},reason,Some(&snapshot)).await?;
            // Recheck both channels at the last possible boundary before issuing a capability.
            if private.load(std::sync::atomic::Ordering::SeqCst)&&business.load(std::sync::atomic::Ordering::SeqCst){dispatch(&adapter,&db,&mut owner,mode,&snapshot).await?;}
            Ok::<_,anyhow::Error>(())
        }.await {
            Ok(())=>{},Err(e)=>{sqlx::query("UPDATE execution.accounts SET snapshot=jsonb_set(COALESCE(snapshot,'{}'),'{connected}','false') WHERE mode=$1").bind(mode.id()).execute(&db).await?;status(&db,mode,"PAUSED",&format!("Ordergateway gepauzeerd: {}",e.to_string().chars().take(300).collect::<String>()),None).await?;}
        }
        tokio::select! {_=wake.notified()=>{},_=tokio::time::sleep(Duration::from_secs(3))=>{}}
    }
}
