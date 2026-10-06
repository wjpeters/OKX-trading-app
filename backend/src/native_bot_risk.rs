//! Exact-version bot capability checks, repeated by the sole exchange gateway.
use crate::order_types::{ExecutionMode,Ticket,Direction,OrderSpec,LimitPolicy};
use crate::order_risk::TradingLimits;
use anyhow::{Result,ensure};
use serde_json::Value;
use rust_decimal::Decimal as D;
pub fn code_version()->String {
 let mut ctx=ring::digest::Context::new(&ring::digest::SHA256);
 for source in [include_str!("neuro.rs"),include_str!("native_bots.rs"),include_str!("native_bot_risk.rs"),include_str!("order_gateway_ledger.rs"),include_str!("bin/okx-orders.rs"),include_str!("order_risk.rs"),include_str!("order_types.rs"),include_str!("gateway_neuro.rs")] {ctx.update(source.as_bytes());}
 ctx.finish().as_ref().iter().map(|b|format!("{b:02x}")).collect()
}
pub fn check_cost_envelope(release:&Value,s:&crate::order_risk::AccountSnapshot)->Result<()>{
 let instrument=release["instrument"].as_str().ok_or_else(||anyhow::anyhow!("Botmarkt ontbreekt"))?;
 let q=s.quotes.get(instrument).ok_or_else(||anyhow::anyhow!("Accountmarkt ontbreekt"))?;use rust_decimal::prelude::ToPrimitive;
 ensure!(q.bid>D::ZERO&&q.ask>=q.bid,"Botkoers ongeldig");
 let fee=crate::order_risk::account_fee(s,instrument)?;let rate=(fee*D::from(2)+(q.ask-q.bid)/q.bid+D::new(1,3)).to_f64().unwrap_or(f64::INFINITY);
 ensure!(rate<=release["qualification"]["execution_rate_ceiling"].as_f64().unwrap_or(-1.),"Actuele kosten vallen buiten de bewezen kostenstress");Ok(())
}
pub fn limits_within(bot:&TradingLimits,account:&TradingLimits,instrument:&str)->Result<()> {
 bot.validate()?;account.validate()?;
 ensure!(bot.account_uid==account.account_uid&&bot.instruments==[instrument]&&account.instruments.iter().any(|i|i==instrument),"Botvrijgave moet aan deze rekening en één bevestigde markt zijn gebonden");
 ensure!([bot.capital<=account.capital,bot.max_order<=account.max_order,bot.max_position<=account.max_position,bot.trade_risk<=account.trade_risk,bot.daily_loss<=account.daily_loss,bot.total_loss<=account.total_loss,bot.drawdown<=account.drawdown,bot.spread_bps<=account.spread_bps,bot.max_orders<=account.max_orders,bot.orders_per_minute<=account.orders_per_minute].iter().all(|b|*b),"Botlimieten mogen de rekeninglimieten niet verruimen");
 ensure!(bot.drawdown<=bot.capital*D::new(5,2),"Botdrawdown blijft maximaal 5% van het bevestigde budget");Ok(())
}
pub fn check_binding(release:&Value,ticket:&Ticket,revision:i64,account:&TradingLimits,active:bool,reducing:bool)->Result<TradingLimits>{
 let limits:TradingLimits=serde_json::from_value(release["limits"].clone())?;
 if reducing {
  ensure!(ticket.side==Direction::Sell&&release["mode"]==ticket.mode.id()&&release["instrument"]==ticket.instrument&&release["account_uid"]==account.account_uid,"Afsluiten vereist eigen rekening en inventaris");
  ensure!(matches!(ticket.order,OrderSpec::Limit{policy:LimitPolicy::Ioc,..}),"Afsluiten vereist prijsbegrensde IOC");return Ok(limits);
 }
 limits_within(&limits,account,&ticket.instrument)?;
 ensure!(release["mode"]==ticket.mode.id()&&release["instrument"]==ticket.instrument&&release["account_uid"]==account.account_uid&&release["policy_revision"].as_i64()==Some(revision),"Botrekening, markt of rekeninglimieten gewijzigd; nieuwe vrijgave vereist");
 ensure!(release["code_version"]==code_version(),"Uitvoeringscode gewijzigd; Demo-acceptatie en botvrijgave opnieuw vereist");
 ensure!(ticket.side==Direction::Sell&&reducing||active&&release["status"]=="ACTIVE","Deze exacte botversie heeft geen actieve vrijgave");
 ensure!(release["qualification"]["historical"]["passed"]==true&&release["qualification"]["forward"]["passed"]==true,"Historisch en forward-bewijs ontbreken voor deze versie");
 ensure!(ticket.mode!=ExecutionMode::Live||release["qualification"]["demo"]["accepted"]==true,"Echte Demo-acceptatie ontbreekt; handmatige toestemming is geen bottoestemming");
 ensure!(matches!(ticket.order,OrderSpec::Limit{policy:LimitPolicy::Ioc,..}),"Neurobots gebruiken alleen direct uitvoerbare orders met vaste prijsgrens");
 Ok(limits)
}
#[cfg(test)] mod tests {
 use super::*;use serde_json::json;
 fn limits()->TradingLimits{serde_json::from_value(json!({"account_uid":"fixture","instruments":["BTC-EUR"],"capital":"500","max_order":"50","max_position":"100","trade_risk":"5","daily_loss":"10","total_loss":"20","drawdown":"25","max_orders":5,"orders_per_minute":5,"spread_bps":"50","confirmed":true})).unwrap()}
 #[test] fn bot_cannot_expand_account_or_drawdown(){let a=limits();let mut b=a.clone();assert!(limits_within(&b,&a,"BTC-EUR").is_ok());b.max_order=D::from(60);assert!(limits_within(&b,&a,"BTC-EUR").is_err());b=a.clone();b.drawdown=D::from(26);assert!(limits_within(&b,&a,"BTC-EUR").is_err());}
 #[test] fn manual_release_never_authorizes_bot_and_code_revision_is_bound(){let a=limits();let t:Ticket=serde_json::from_value(json!({"mode":"LIVE","instrument":"BTC-EUR","side":"buy","quantity":"0.01","order":{"type":"limit","price":"100","policy":"ioc"},"protection":{"stop":"99","target":null}})).unwrap();let mut r=json!({"mode":"LIVE","instrument":"BTC-EUR","account_uid":"fixture","policy_revision":2,"code_version":code_version(),"status":"ACTIVE","limits":a,"qualification":{"historical":{"passed":true},"forward":{"passed":true},"demo":{"accepted":false}}});assert!(check_binding(&r,&t,2,&a,true,false).is_err());r["qualification"]["demo"]["accepted"]=json!(true);assert!(check_binding(&r,&t,2,&a,true,false).is_ok());assert!(check_binding(&r,&t,3,&a,true,false).is_err());r["code_version"]=json!("old");assert!(check_binding(&r,&t,2,&a,true,false).is_err());}
}
