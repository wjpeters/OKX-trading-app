//! JSON-lines IPC. No HTTP, database, settings or exchange modules are imported.
#[path="../model.rs"] mod model;
#[path="../neuro.rs"] mod neuro;
#[path="../neuro_data.rs"] mod neuro_data;
use anyhow::{Result,ensure};
use serde_json::{Value,json};
use std::{io::{self,BufRead,Write},path::Path};
fn run(v:Value,series:&mut Option<neuro::Series>)->Result<Value>{
 match v["command"].as_str().unwrap_or(""){
 "load"=>{let path=v["path"].as_str().ok_or_else(||anyhow::anyhow!("Pad ontbreekt"))?;ensure!(Path::new(path).is_absolute(),"Datasetpad moet absoluut zijn");let mut s=neuro_data::load(Path::new(path),v["bar"].as_str().unwrap_or("1m"))?;if let Some(cache)=v["laya_path"].as_str(){s.laya=neuro_data::laya_map(&serde_json::from_slice(&std::fs::read(cache)?)?)?;}let r=json!({"bars":s.candles.len(),"start_ms":s.candles.first().unwrap().ts,"end_ms":s.candles.last().unwrap().ts+s.step,"feature_version":neuro::FEATURES});*series=Some(s);Ok(r)},
 "fit"=>{let s=series.as_ref().ok_or_else(||anyhow::anyhow!("Geen dataset"))?;Ok(serde_json::to_value(s.fit(v["start_ms"].as_i64().unwrap_or(0),v["end_ms"].as_i64().unwrap_or(0))?)?)},
 "profile"=>{let s=series.as_ref().ok_or_else(||anyhow::anyhow!("Geen dataset"))?;let a:neuro::Artifact=serde_json::from_value(v["artifact"].clone())?;a.validate()?;let costs:neuro::Costs=serde_json::from_value(v["costs"].clone())?;costs.validate()?;let mut sim=neuro::Simulation::new(&costs);let mut times=vec![];for i in neuro::WARMUP..s.candles.len(){let t=std::time::Instant::now();sim.step(s,i,&a,&costs,"neural",true)?;times.push(t.elapsed().as_micros()as u64);}times.sort_unstable();ensure!(!times.is_empty(),"Geen profieldata");Ok(json!({"samples":times.len(),"strategy_p95_us":times[(times.len()-1)*95/100],"scope":"shared Rust strategy step; excludes Laya, database and exchange ACK"}))},
 "features"=>{let s=series.as_ref().ok_or_else(||anyhow::anyhow!("Geen dataset"))?;let after=v["after"].as_i64().unwrap_or(0);let end=v["end_ms"].as_i64().unwrap_or(i64::MAX);let limit=v["limit"].as_u64().unwrap_or(64).min(256)as usize;Ok(json!(s.candles.iter().enumerate().filter(|(i,c)|*i>=neuro::WARMUP&&c.ts>=after&&c.ts<end).take(limit).map(|(i,c)|json!({"ts":c.ts,"features":s.raw[i]})).collect::<Vec<_>>()))},
 "evaluate"=>{let s=series.as_ref().ok_or_else(||anyhow::anyhow!("Geen dataset"))?;neuro::evaluate(s,&serde_json::from_value(v["artifact"].clone())?,&serde_json::from_value(v["costs"].clone())?,v["start_ms"].as_i64().unwrap_or(0),v["end_ms"].as_i64().unwrap_or(0),v["mode"].as_str().unwrap_or("neural"))},
 _=>Err(anyhow::anyhow!("Onbekende evaluatoropdracht"))
 }
}
fn main()->Result<()> {let stdin=io::stdin();let mut out=io::BufWriter::new(io::stdout());let mut series=None;for line in stdin.lock().lines(){let line=line?;if line.len()>2_000_000{writeln!(out,"{{\"error\":\"Opdracht te groot\"}}")?;out.flush()?;continue;}let r=serde_json::from_str(&line).map_err(anyhow::Error::from).and_then(|v|run(v,&mut series));let value=match r{Ok(v)=>v,Err(e)=>json!({"error":e.to_string()})};writeln!(out,"{}",serde_json::to_string(&value)?)?;out.flush()?;}Ok(())}
