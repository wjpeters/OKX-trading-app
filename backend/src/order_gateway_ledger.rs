use super::*;
pub async fn fills(
    okx: &exchange::Okx,
    db: &sqlx::PgPool,
    mode: ExecutionMode,
    parent: &str,
    external: &str,
    expected: D,
) -> Result<()> {
    let mut after = None;
    let mut pages = 0;
    loop {
        let suffix = after
            .as_ref()
            .map(|id| format!("&after={id}"))
            .unwrap_or_default();
        let v = okx
            .get(&format!(
                "/api/v5/trade/fills?instType=SPOT&ordId={external}&limit=100{suffix}"
            ))
            .await?;
        let list = rows(&v)?;
        for r in list {
            let f = order_ledger::Fill::parse(r)?;
            ensure!(
                f.exchange_id == external,
                "Onverwachte fill-orderidentiteit"
            );
            sqlx::query("INSERT INTO execution.fills(mode,bill_id,parent_id,exchange_id,instrument,side,quantity,price,fee,fee_currency,fill_ms,detail) VALUES($1,$2,$3,$4,$5,$6,$7::numeric,$8::numeric,$9::numeric,$10,$11,$12) ON CONFLICT(mode,bill_id) DO NOTHING")
                .bind(mode.id()).bind(&f.bill_id).bind(parent).bind(external).bind(&f.instrument).bind(if f.side==Direction::Buy{"buy"}else{"sell"}).bind(f.quantity.to_string()).bind(f.price.to_string()).bind(f.fee.to_string()).bind(&f.fee_currency).bind(f.fill_ms).bind(r).execute(db).await?;
        }
        if list.len() < 100 {
            break;
        }
        pages += 1;
        ensure!(
            pages < 50,
            "Fill-historie te groot; handmatige reconciliatie nodig"
        );
        let next = list
            .last()
            .and_then(|r| r["billId"].as_str())
            .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
            .ok_or_else(|| anyhow::anyhow!("Fillpaginering ontbreekt"))?
            .to_string();
        ensure!(after.as_ref() != Some(&next), "Fillpaginering herhaalt");
        after = Some(next);
    }
    let total:String=sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::text FROM execution.fills WHERE mode=$1 AND exchange_id=$2").bind(mode.id()).bind(external).fetch_one(db).await?;
    ensure!(
        total.parse::<D>()? == expected,
        "Fill-ledger komt nog niet overeen met OKX; nieuwe orders wachten"
    );
    Ok(())
}
pub async fn bills(okx: &exchange::Okx, db: &sqlx::PgPool, mode: ExecutionMode) -> Result<()> {
    let mut after = None;
    let mut pages = 0;
    loop {
        let suffix = after
            .as_ref()
            .map(|id| format!("&after={id}"))
            .unwrap_or_default();
        let v = okx
            .get(&format!("/api/v5/account/bills?limit=100{suffix}"))
            .await?;
        let list = rows(&v)?;
        let mut duplicates = 0;
        for r in list {
            let id = r["billId"]
                .as_str()
                .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| anyhow::anyhow!("Boeking-id ontbreekt"))?;
            let inserted=sqlx::query("INSERT INTO execution.bills(mode,bill_id,currency,balance_change,kind,bill_ms,detail) VALUES($1,$2,$3,$4::numeric,$5,$6,$7) ON CONFLICT(mode,bill_id) DO NOTHING")
                .bind(mode.id()).bind(id).bind(r["ccy"].as_str().ok_or_else(||anyhow::anyhow!("Boekingsvaluta ontbreekt"))?).bind(dec(&r["balChg"])?.to_string()).bind(r["type"].as_str().unwrap_or("unknown")).bind(r["ts"].as_str().unwrap_or("0").parse::<i64>()?).bind(r).execute(db).await?.rows_affected();
            if inserted == 0 {
                duplicates += 1;
            }
        }
        if list.len() < 100 || duplicates == list.len() {
            break;
        }
        pages += 1;
        ensure!(pages < 50, "Accountboekingen vereisen langere historie");
        let next = list.last().unwrap()["billId"].as_str().unwrap().to_string();
        ensure!(after.as_ref() != Some(&next), "Boekingspaginering herhaalt");
        after = Some(next);
    }
    Ok(())
}
pub async fn protective_orders(
    okx: &exchange::Okx,
    db: &sqlx::PgPool,
    mode: ExecutionMode,
) -> Result<()> {
    let protects=sqlx::query("SELECT p.client_id,p.parent_id,p.state,p.quantity::text,p.cancel_requested,o.ticket FROM execution.protection p JOIN execution.orders o ON o.mode=p.mode AND o.client_id=p.parent_id WHERE p.mode=$1 AND p.state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).fetch_all(db).await?;
    for row in protects {
        let id: String = row.get("client_id");
        let parent: String = row.get("parent_id");
        let t: Ticket = serde_json::from_value(row.get("ticket"))?;
        let v = match okx
            .get(&format!("/api/v5/trade/order-algo?algoClOrdId={id}"))
            .await
        {
            Ok(v) => v,
            Err(_) => {
                sqlx::query("UPDATE execution.protection SET state='UNKNOWN' WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).execute(db).await?;
                continue;
            }
        };
        let d = rows(&v)?
            .first()
            .ok_or_else(|| anyhow::anyhow!("Beschermingsdetail ontbreekt"))?;
        let external = d["algoId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Beschermings-id ontbreekt"))?;
        let mut all = true;
        let mut children = false;
        let mut exited = D::ZERO;
        if let Some(ids) = d["ordIdList"].as_array() {
            for child in ids {
                let child = child
                    .as_str()
                    .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
                    .ok_or_else(|| anyhow::anyhow!("Child-id ontbreekt"))?;
                sqlx::query("INSERT INTO execution.children(mode,parent_id,exchange_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(mode.id()).bind(&parent).bind(child).execute(db).await?;
                let v = okx
                    .get(&format!(
                        "/api/v5/trade/order?instId={}&ordId={child}",
                        t.instrument
                    ))
                    .await?;
                let r = rows(&v)?
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("Beschermingsfill ontbreekt"))?;
                let fill = optional(&r["accFillSz"])?;
                if fill > D::ZERO {
                    fills(okx, db, mode, &parent, child, fill).await?;
                }
                exited += fill;
                all &= ["filled", "canceled"].iter().any(|s| r["state"] == *s);
                children = true;
            }
        }
        let state = if d["state"] == "canceled" {
            "CANCELED"
        } else if d["state"] == "order_failed" {
            "REJECTED"
        } else if children && all && exited == row.get::<String, _>("quantity").parse::<D>()? {
            "FILLED"
        } else if children && all {
            "REJECTED"
        } else if d["state"] == "effective" {
            "TRIGGERED"
        } else {
            "ACTIVE"
        };
        sqlx::query("UPDATE execution.protection SET state=$3,exchange_id=$4,result=$5 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).bind(state).bind(external).bind(&v).execute(db).await?;
        let readiness:Option<Value>=sqlx::query_scalar("SELECT snapshot FROM execution.accounts WHERE mode=$1").bind(mode.id()).fetch_one(db).await?;
        let can_close=readiness.as_ref().is_some_and(|s|s["connected"]==true&&s["reconciled"]==true&&s["received_ms"].as_i64().is_some_and(|ms|(0..=15000).contains(&(now()-ms))));
        if row.get::<bool,_>("cancel_requested") && can_close && ["ACTIVE","ACCEPTED"].contains(&state) {
            sqlx::query("UPDATE execution.protection SET state='CANCEL_PENDING' WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).execute(db).await?;
            if okx.cancel_native(&t.instrument,external,true).await.is_err(){sqlx::query("UPDATE execution.protection SET state='UNKNOWN' WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&id).execute(db).await?;}
        }
        if ["CANCELED", "REJECTED"].contains(&state) && !row.get::<bool,_>("cancel_requested") {
            anyhow::bail!(
                "Bescherming {id} is weggevallen; account geblokkeerd, herstel of sluit blootstelling bij OKX"
            );
        }
    }
    Ok(())
}
pub async fn protect(okx: &exchange::Okx, db: &sqlx::PgPool, mode: ExecutionMode) -> Result<bool> {
    let parents=sqlx::query("SELECT o.client_id,o.ticket,o.exchange_id,o.compiled,o.close_requested FROM execution.orders o WHERE o.mode=$1 AND o.ticket->>'side'='buy' AND EXISTS(SELECT 1 FROM execution.fills f WHERE f.mode=o.mode AND f.parent_id=o.client_id AND f.side='buy')").bind(mode.id()).fetch_all(db).await?;
    let mut all_protected = true;
    for row in parents {
        let id: String = row.get("client_id");
        let t: Ticket = serde_json::from_value(row.get("ticket"))?;
        let base = t.instrument.split('-').next().unwrap();
        let net:String=sqlx::query_scalar("SELECT COALESCE(sum(CASE WHEN side='buy' THEN quantity ELSE -quantity END+CASE WHEN fee_currency=$3 THEN fee ELSE 0 END),0)::text FROM execution.fills WHERE mode=$1 AND parent_id=$2").bind(mode.id()).bind(&id).bind(base).fetch_one(db).await?;
        let committed:String=sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::text FROM execution.protection WHERE mode=$1 AND parent_id=$2 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).bind(&id).fetch_one(db).await?;
        let net: D = net.parse()?;
        let committed: D = committed.parse()?;
        if row.get::<bool,_>("close_requested") {
            if net>committed {
                let meta=okx.get(&format!("/api/v5/account/instruments?instType=SPOT&instId={}",t.instrument)).await?;
                let lot=dec(&rows(&meta)?.first().ok_or_else(||anyhow::anyhow!("Afsluitmarkt ontbreekt"))?["lotSz"])?;
                if net-committed>=lot {all_protected=false;}
            }
            continue;
        }
        let bad:i64=sqlx::query_scalar("SELECT count(*) FROM execution.protection WHERE mode=$1 AND parent_id=$2 AND state IN ('UNKNOWN','SUBMITTING','REJECTED','CANCELED')").bind(mode.id()).bind(&id).fetch_one(db).await?;
        if bad > 0 {
            all_protected = false;
            continue;
        }
        if net <= committed {
            continue;
        }
        let meta = okx
            .get(&format!(
                "/api/v5/account/instruments?instType=SPOT&instId={}",
                t.instrument
            ))
            .await?;
        let meta = rows(&meta)?
            .first()
            .ok_or_else(|| anyhow::anyhow!("Beschermingsmarkt ontbreekt"))?;
        let rules = MarketRules {
            base: base.into(),
            quote: t.instrument.split_once('-').unwrap().1.into(),
            tick: dec(&meta["tickSz"])?,
            lot: dec(&meta["lotSz"])?,
            minimum: dec(&meta["minSz"])?,
        };
        let (qty, unprotected) =
            order_ledger::protection_quantity(net, committed, rules.lot, rules.minimum)?;
        if qty == D::ZERO {
            if unprotected >= rules.lot {
                all_protected = false;
                event(db,mode,&id,"UNPROTECTED_DUST",&json!({"quantity":unprotected.to_string(),"minimum":rules.minimum.to_string()})).await?;
            }
            continue;
        }
        let seq: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM execution.protection WHERE mode=$1 AND parent_id=$2",
        )
        .bind(mode.id())
        .bind(&id)
        .fetch_one(db)
        .await?;
        let pid = format!("P{id}{seq:X}");
        ensure!(
            pid.len() <= 32,
            "Te veel beschermingsdelen; nieuwe orders geblokkeerd"
        );
        let auth = authorize_protection(&t, &rules, net, committed, &pid, now())?;
        sqlx::query("INSERT INTO execution.protection(mode,parent_id,client_id,quantity,state,created_ms) VALUES($1,$2,$3,$4::numeric,'SUBMITTING',$5)").bind(mode.id()).bind(&id).bind(&pid).bind(qty.to_string()).bind(now()).execute(db).await?;
        match okx.send_authorized(&auth).await {
            Ok(v) => {
                let external = v["data"][0]["algoId"].as_str().unwrap();
                sqlx::query("UPDATE execution.protection SET state='ACCEPTED',exchange_id=$3,result=$4 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&pid).bind(external).bind(&v).execute(db).await?;
                all_protected = false;
            }
            Err(e) => {
                let state = if e.to_string().starts_with("OKX_REJECTED:") {
                    "REJECTED"
                } else {
                    "UNKNOWN"
                };
                sqlx::query("UPDATE execution.protection SET state=$3,result=$4 WHERE mode=$1 AND client_id=$2").bind(mode.id()).bind(&pid).bind(state).bind(json!({"reason":e.to_string()})).execute(db).await?;
                sqlx::query("UPDATE execution.orders SET command='CANCEL' WHERE mode=$1 AND client_id=$2 AND state NOT IN ('FILLED','CANCELED','REJECTED')").bind(mode.id()).bind(&id).execute(db).await?;
                all_protected = false;
                event(db,mode,&id,"PROTECTION_FAILURE",&json!({"quantity":qty.to_string(),"state":state,"action":"Nieuwe aankopen blokkeren, parent annuleren, resterende blootstelling zelf bij OKX beoordelen"})).await?;
            }
        }
    }
    Ok(all_protected)
}
