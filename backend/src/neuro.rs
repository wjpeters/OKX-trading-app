//! Credential-free, causal strategy kernel shared by research and forward execution.
use crate::model::Candle;
use anyhow::{Result, ensure};
use rust_decimal::{Decimal as D, prelude::ToPrimitive};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const INPUTS: usize = 16;
pub const HIDDEN: usize = 16;
pub const OUTPUTS: usize = 4;
pub const WEIGHTS: usize = INPUTS * HIDDEN + HIDDEN + HIDDEN * OUTPUTS + OUTPUTS;
pub const FEATURES: &str = "causal-ohlcv-v1";
pub const WARMUP: usize = 200;
pub const LABELS: [&str; INPUTS] = ["Rendement 1", "Rendement 5", "Volatiliteit", "Trend", "RSI", "Volume", "Bollinger z", "ATR", "Positie", "Open resultaat", "Houdduur", "Kosten", "Laya trend", "Laya range", "Laya volatiel", "Laya zekerheid"];
pub const STRATEGIES: [&str; 4] = ["trend", "breakout", "mean_reversion", "hold"];
fn d(n: f64) -> D { D::from_f64_retain(n).unwrap_or(D::ZERO) }
fn f(n: D) -> f64 { n.to_f64().unwrap_or(0.) }
fn one() -> D { D::ONE }
fn default_fee() -> D { D::new(10, 4) }
fn default_spread() -> f64 { 10. }
fn default_slip() -> f64 { 5. }
fn default_participation() -> f64 { 0.01 }
fn default_latency() -> i64 { 500 }
fn default_capital() -> D { D::from(10000) }
fn default_allocation() -> D { D::new(1,1) }
fn default_dd() -> f64 { 0.05 }
fn default_lot() -> D { D::new(1,8) }
fn default_tick() -> D { D::new(1,2) }
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Costs {
    #[serde(default="default_fee")] pub taker_fee: D,
    #[serde(default)] pub fee_in_base: bool,
    #[serde(default="default_spread")] pub spread_bps: f64,
    #[serde(default="default_slip")] pub slippage_bps: f64,
    #[serde(default)] pub impact_bps: f64,
    #[serde(default="default_participation")] pub participation: f64,
    #[serde(default="default_latency")] pub latency_ms: i64,
    #[serde(default="default_capital")] pub capital: D,
    #[serde(default="default_allocation")] pub allocation: D,
    #[serde(default="default_dd")] pub max_drawdown: f64,
    #[serde(default="default_lot")] pub lot: D,
    #[serde(default="default_tick")] pub tick: D,
    #[serde(default="default_lot")] pub minimum: D,
    #[serde(default)] pub ai_cost_per_decision: D,
    #[serde(default)] pub other_cost_per_roundtrip: D,
    #[serde(default="one")] pub stress: D,
    #[serde(default)] pub source: String,
}
impl Default for Costs { fn default()->Self { serde_json::from_value(json!({})).unwrap() } }
impl Costs {
 pub fn validate(&self)->Result<()> {
    ensure!((-0.001..=0.02).contains(&f(self.taker_fee)) && [self.spread_bps,self.slippage_bps,self.impact_bps].iter().all(|v|v.is_finite()&&(0.0..=500.).contains(v)),"Kosten ongeldig");
    ensure!(self.capital>D::ZERO && self.capital<=D::from(1000000) && self.allocation>D::ZERO && self.allocation<=D::new(25,2),"Onderzoeksbudget ongeldig");
    ensure!(self.participation.is_finite()&&(0.00001..=0.05).contains(&self.participation) && (0..=60000).contains(&self.latency_ms),"Uitvoeringsgrenzen ongeldig");
    ensure!(self.max_drawdown.is_finite()&&(0.0001..=0.05).contains(&self.max_drawdown) && self.lot>D::ZERO && self.tick>D::ZERO && self.minimum>=self.lot,"Vaste risicogrenzen ongeldig");
    ensure!(self.ai_cost_per_decision>=D::ZERO && self.other_cost_per_roundtrip>=D::ZERO && self.stress>=D::ONE && self.stress<=D::from(3),"Kostentoerekening ongeldig");
    Ok(())
 }
 pub fn bps(&self)->D { d((self.spread_bps/2.+self.slippage_bps+self.impact_bps)/10000.)*self.stress }
 pub fn buy_price(&self,raw:D)->D { ((raw*(D::ONE+self.bps()))/self.tick).ceil()*self.tick }
 pub fn sell_price(&self,raw:D)->D { ((raw*(D::ONE-self.bps()))/self.tick).floor()*self.tick }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters { pub fast:usize, pub slow:usize, pub lookback:usize, pub rsi_low:f64, pub bb_z:f64, pub volume_ratio:f64, pub min_atr:f64, pub stop_bps:f64, pub target_bps:f64, pub max_hold:usize, pub cooldown:usize, pub confidence:f64 }
impl Default for Parameters {fn default()->Self {Self{fast:9,slow:40,lookback:20,rsi_low:30.,bb_z:2.,volume_ratio:1.2,min_atr:0.0005,stop_bps:100.,target_bps:200.,max_hold:48,cooldown:5,confidence:0.3}}}
impl Parameters {pub fn validate(&self)->Result<()> {
 ensure!((2..=30).contains(&self.fast)&&(31..=100).contains(&self.slow)&&(10..=100).contains(&self.lookback),"Indicatorperiode buiten zoekruimte");
 for (n,lo,hi) in [(self.rsi_low,10.,45.),(self.bb_z,1.,3.),(self.volume_ratio,0.5,3.),(self.min_atr,0.,0.02),(self.stop_bps,25.,300.),(self.target_bps,30.,600.),(self.confidence,0.25,0.8)] {ensure!(n.is_finite()&&(lo..=hi).contains(&n),"Strategieparameter buiten zoekruimte");}
 ensure!((2..=288).contains(&self.max_hold)&&self.cooldown<=100,"Houdduur/cooldown buiten zoekruimte"); Ok(())
}}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Normalizer { pub mean:[f64;INPUTS], pub scale:[f64;INPUTS], pub start_ms:i64, pub end_ms:i64 }
impl Default for Normalizer {fn default()->Self {Self{mean:[0.;INPUTS],scale:[1.;INPUTS],start_ms:0,end_ms:0}}}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Artifact {
 pub schema:u32, pub feature_version:String, pub weights:Vec<f64>, pub parameters:Parameters,
 pub normalizer:Normalizer, pub laya:bool, pub laya_checkpoint:String,
 #[serde(default)] pub laya_runtime:String,
 pub instrument:String, pub bar:String, pub dataset_hash:String,
 pub seed:u32, pub generation:u32,
}
impl Artifact {pub fn validate(&self)->Result<()> {
 ensure!(self.schema==1&&self.feature_version==FEATURES&&self.weights.len()==WEIGHTS,"Onbekend netwerk/featureprofiel");
 ensure!(self.weights.iter().all(|w|w.is_finite()&&w.abs()<=8.),"Netwerkgewichten buiten grens");
 ensure!(self.normalizer.mean.iter().all(|n|n.is_finite())&&self.normalizer.scale.iter().all(|n|n.is_finite()&&*n>0.),"Normalisatie ongeldig");
 ensure!(["1m","5m","15m"].contains(&self.bar.as_str())&&(self.instrument.ends_with("-EUR")||self.instrument.ends_with("-USDC")),"Onderzoeksmarkt/timeframe ongeldig");
 ensure!(!self.laya||self.laya_checkpoint=="7b928d828b7b0e022f929d9bd2e44165aa270148","Laya-checkpoint moet vastgezet zijn"); self.parameters.validate()
}}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Activation {pub inputs:[f64;INPUTS],pub hidden:[f64;HIDDEN],pub outputs:[f64;OUTPUTS],pub selected:usize}
pub fn activate(a:&Artifact,raw:[f64;INPUTS])->Activation {
 let inputs=std::array::from_fn(|i|((raw[i]-a.normalizer.mean[i])/a.normalizer.scale[i]).clamp(-8.,8.));
 let w=&a.weights;
 let hidden=std::array::from_fn(|j|(w[256+j]+(0..INPUTS).map(|i|inputs[i]*w[j*INPUTS+i]).sum::<f64>()).tanh());
 let logits:[f64;OUTPUTS]=std::array::from_fn(|j|w[336+j]+(0..HIDDEN).map(|i|hidden[i]*w[272+j*HIDDEN+i]).sum::<f64>());
 let max=logits.iter().copied().fold(f64::NEG_INFINITY,f64::max);
 let exps=logits.map(|n|(n-max).exp());let sum=exps.iter().sum::<f64>();let outputs=exps.map(|n|n/sum);
 let selected=(0..OUTPUTS).max_by(|i,j|outputs[*i].total_cmp(&outputs[*j])).unwrap();
 Activation{inputs,hidden,outputs,selected:if outputs[selected]<a.parameters.confidence{3}else{selected}}
}
#[derive(Clone)]
pub struct Series {pub candles:Vec<Candle>,pub raw:Vec<[f64;INPUTS]>,pub laya: BTreeMap<i64,[f64;4]>,pub step:i64}
impl Series {
 pub fn new(cs:Vec<Candle>,step:i64)->Result<Self> {
  ensure!(!cs.is_empty()&&cs.len()<=2_000_000&&[60000,300000,900000].contains(&step),"Datasetgrootte/timeframe ongeldig");
  ensure!(cs.iter().all(|c|c.closed&&c.ts%step==0&&c.low>D::ZERO&&c.low<=c.open&&c.low<=c.close&&c.high>=c.open&&c.high>=c.close&&c.volume>=D::ZERO),"Ongeldige gesloten candles");
  ensure!(cs.windows(2).all(|w|w[1].ts-w[0].ts==step),"Dataset bevat datagaten of duplicaten");
  let mut raw=Vec::with_capacity(cs.len());
  for (i,c) in cs.iter().enumerate(){
   let close=f(c.close);let start=i.saturating_sub(19);let win=&cs[start..=i];let n=win.len()as f64;
   let mean=win.iter().map(|x|f(x.close)).sum::<f64>()/n;
   let sd=(win.iter().map(|x|(f(x.close)-mean).powi(2)).sum::<f64>()/n).sqrt();
   let vmean=win.iter().map(|x|f(x.volume)).sum::<f64>()/n;
   let mut gains=0.;let mut losses=0.;let mut tr=0.;let mut rets=vec![];
   for j in start.max(1)..=i {let p=f(cs[j-1].close);let delta=f(cs[j].close)-p;gains+=delta.max(0.);losses+=(-delta).max(0.);tr+=(f(cs[j].high)-f(cs[j].low)).max((f(cs[j].high)-p).abs()).max((f(cs[j].low)-p).abs());rets.push(delta/p);}
   let rsi=if gains+losses>0.{gains/(gains+losses)}else{0.5};
   let vol=if !rets.is_empty(){(rets.iter().map(|x|x*x).sum::<f64>()/rets.len()as f64).sqrt()}else{0.};
   raw.push([if i>0{close/f(cs[i-1].close)-1.}else{0.},if i>=5{close/f(cs[i-5].close)-1.}else{0.},vol,(tail_ema(&cs,i,9)-tail_ema(&cs,i,26))/close,rsi,if vmean>0.{f(c.volume)/vmean}else{0.},if sd>1e-12{(close-mean)/sd}else{0.},tr/n/close,0.,0.,0.,0.,0.,0.,0.,0.]);
  }
  Ok(Self{candles:cs,raw,laya:BTreeMap::new(),step})
 }
 pub fn fit(&self,start:i64,end:i64)->Result<Normalizer>{
  let xs:Vec<_>=self.candles.iter().enumerate().filter(|(i,c)|*i>=WARMUP&&c.ts>=start&&c.ts<end).map(|(i,_)|self.raw[i]).collect();
  ensure!(xs.len()>WARMUP,"Te weinig trainingsdata voor normalisatie");
  let mut mean=[0.;INPUTS];let mut scale=[1.;INPUTS];
  for j in 0..8 {mean[j]=xs.iter().map(|x|x[j]).sum::<f64>()/xs.len()as f64;scale[j]=(xs.iter().map(|x|(x[j]-mean[j]).powi(2)).sum::<f64>()/xs.len()as f64).sqrt().max(1e-8);}
  Ok(Normalizer{mean,scale,start_ms:start,end_ms:end})
 }
}
pub fn aggregate(cs:&[Candle],step:i64)->Result<Vec<Candle>> {
 ensure!([60000,300000,900000].contains(&step),"Timeframe ongeldig");
 ensure!(cs.windows(2).all(|w|w[1].ts-w[0].ts==60000),"1m-dataset bevat gaten");
 let count=step/60000;let mut out=vec![];let mut i=0;
 while i<cs.len(){if cs[i].ts%step!=0{i+=1;continue;}let end=i+count as usize;if end>cs.len(){break;}
  let xs=&cs[i..end];ensure!(xs.iter().all(|c|c.closed),"Open candle in dataset");
  out.push(Candle{ts:xs[0].ts,open:xs[0].open,high:xs.iter().map(|c|c.high).max().unwrap(),low:xs.iter().map(|c|c.low).min().unwrap(),close:xs.last().unwrap().close,volume:xs.iter().map(|c|c.volume).sum(),closed:true});i=end;}
 Ok(out)
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Position {pub quantity:D,pub debit:D,pub entry:D,pub stop:D,pub target:D,pub strategy:usize,pub age:usize,pub opened_ms:i64}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Simulation {
 pub cash:D,pub position:Option<Position>,pub pending:Option<usize>,pub cooldown:usize,pub high_water:D,
 pub max_drawdown:f64,pub fees:D,pub execution_costs:D,pub ai_costs:D,pub other_costs:D,
 pub trades:usize,pub wins:D,pub losses:D,pub exposed:usize,pub bars:usize,pub halted:bool,pub last_ms:i64,
 pub daily:BTreeMap<i64,D>,pub last_day:i64,pub last_equity:D,
 pub curve:Vec<Value>,pub decisions:Vec<Value>,pub last_activation:Option<Activation>, #[serde(default)] pub last_decision:Option<Value>,
}
impl Simulation {
 pub fn new(costs:&Costs)->Self {Self{cash:costs.capital,position:None,pending:None,cooldown:0,high_water:costs.capital,max_drawdown:0.,fees:D::ZERO,execution_costs:D::ZERO,ai_costs:D::ZERO,other_costs:D::ZERO,trades:0,wins:D::ZERO,losses:D::ZERO,exposed:0,bars:0,halted:false,last_ms:0,daily:BTreeMap::new(),last_day:0,last_equity:costs.capital,curve:vec![],decisions:vec![],last_activation:None,last_decision:None}}
 fn equity(&self,price:D,costs:&Costs)->D {self.cash+self.position.as_ref().map(|p|p.quantity*costs.sell_price(price)*(D::ONE-costs.taker_fee*costs.stress)-(costs.other_cost_per_roundtrip*costs.stress)).unwrap_or(D::ZERO)}
 fn exit(&mut self,price:D,raw:D,costs:&Costs,ts:i64,reason:&str,force:bool,volume:D){
  let Some(mut p)=self.position.take()else{return};
  let available=(volume*d(costs.participation)/costs.lot).floor()*costs.lot;
  let quantity=if force{p.quantity}else{p.quantity.min(available)};
  if quantity<=D::ZERO {self.position=Some(p);return;}
  // A base-currency sell fee consumes base in addition to the sold amount.
  let fee_rate=costs.taker_fee*costs.stress;
  let sold=if costs.fee_in_base{quantity/(D::ONE+fee_rate)}else{quantity};
  let fee=if costs.fee_in_base{(quantity-sold)*price}else{sold*price*fee_rate};
  let proceeds=if costs.fee_in_base{sold*price}else{sold*price-fee};
  let proportion=quantity/p.quantity;let debit=p.debit*proportion;
  self.cash+=proceeds;self.fees+=fee;self.execution_costs+=sold*(raw-price);
  p.quantity-=quantity;p.debit-=debit;
  if p.quantity<=D::ZERO {
   self.cash-=(costs.other_cost_per_roundtrip*costs.stress);self.other_costs+=(costs.other_cost_per_roundtrip*costs.stress);
   let net=proceeds-debit-(costs.other_cost_per_roundtrip*costs.stress);
   // Accumulate partial realizations with the roundtrip before classifying profit factor.
   let previous=self.decisions.iter().rev().take_while(|e|e["opened_ms"].as_i64()==Some(p.opened_ms)).filter_map(|e|e["partial_net"].as_str().and_then(|x|x.parse::<D>().ok())).sum::<D>();
   let total=net+previous;if total>=D::ZERO{self.wins+=total}else{self.losses-=total};self.trades+=1;
   self.decisions.push(json!({"ts":ts,"action":"sell","strategy":STRATEGIES[p.strategy],"reason":reason,"net":total.to_string(),"opened_ms":p.opened_ms}));
  }else{self.decisions.push(json!({"ts":ts,"action":"partial_exit","partial_net":(proceeds-debit).to_string(),"opened_ms":p.opened_ms}));self.position=Some(p);}
 }
 pub fn step(&mut self,s:&Series,i:usize,a:&Artifact,costs:&Costs,mode:&str,allow_entry:bool)->Result<()> {
  let c=&s.candles[i];ensure!(c.ts>self.last_ms,"Candle al verwerkt of terug in tijd");
  self.last_ms=c.ts;self.bars+=1;if self.cooldown>0{self.cooldown-=1;}
  if let Some(strategy)=self.pending.take(){if self.position.is_none()&&!self.halted&&allow_entry{
   let raw=c.open+(c.high-c.open)*d(costs.latency_ms as f64/s.step as f64);let price=costs.buy_price(raw);
   let capacity=c.volume*d(costs.participation);let budget=(costs.capital*costs.allocation).min(self.cash);
   let qty=((budget/(price*(D::ONE+costs.taker_fee*costs.stress))).min(capacity)/costs.lot).floor()*costs.lot;
   if qty>=costs.minimum {
    let fee=qty*price*costs.taker_fee*costs.stress;let debit=if costs.fee_in_base{qty*price}else{qty*price+fee};
    let received=if costs.fee_in_base{qty*(D::ONE-costs.taker_fee*costs.stress)}else{qty};
    self.cash-=debit;self.fees+=fee;self.execution_costs+=qty*(price-c.open);
    self.position=Some(Position{quantity:received,debit,entry:price,stop:price*(D::ONE-d(a.parameters.stop_bps/10000.)),target:price*(D::ONE+d(a.parameters.target_bps/10000.)),strategy,age:0,opened_ms:c.ts});
    self.decisions.push(json!({"ts":c.ts,"action":"buy","strategy":STRATEGIES[strategy],"quantity":received.to_string(),"reason":"Gesloten candle, signaalfilter en vaste risicokern geslaagd"}));
   }
  }}
  if let Some(p)=&mut self.position {p.age+=1;self.exposed+=1;let stop=p.stop;let target=p.target;let aged=p.age>=a.parameters.max_hold;
   if mode!="buy_and_hold"&&c.low<=stop {self.exit(costs.sell_price(c.open.min(stop)),c.open.min(stop),costs,c.ts,"stop",false,c.volume);self.cooldown=a.parameters.cooldown;}
   else if mode!="buy_and_hold"&&c.high>=target {self.exit(costs.sell_price(target),target,costs,c.ts,"target",false,c.volume);self.cooldown=a.parameters.cooldown;}
   else if mode!="buy_and_hold"&&aged {self.exit(costs.sell_price(c.close),c.close,costs,c.ts,"houdduur",false,c.volume);self.cooldown=a.parameters.cooldown;}
  }
  let raw=decision_inputs(s,i,self.position.as_ref().map(|p|p.entry),self.position.as_ref().map(|p|p.age).unwrap_or(0),a.parameters.max_hold,f(costs.taker_fee*costs.stress)*2.+f(costs.bps())*2.,a.laya);
  let laya=s.laya.get(&c.ts);
  let act=activate(a,raw);let chosen=match mode {"trend"|"buy_and_hold"=>0,"breakout"=>1,"mean_reversion"=>2,"cash"=>3,_=>act.selected};
  let p=&a.parameters;
  let signal=if i<WARMUP{false}else if mode=="buy_and_hold"{self.trades==0}else{entry_signal(s,i,p,chosen,&raw)}
   &&allow_entry&&!self.halted&&self.cooldown==0&&self.position.is_none()&&(!a.laya||mode!="neural"||laya.is_some());
  if signal{self.pending=Some(chosen);}
  self.ai_costs+=costs.ai_cost_per_decision;self.cash-=costs.ai_cost_per_decision;
  self.last_activation=Some(act);
  let equity=self.equity(c.close,costs);self.high_water=self.high_water.max(equity);
  // Worst adverse mark includes open inventory and estimated liquidation costs.
  let adverse=self.equity(c.low,costs);let dd=f((self.high_water-adverse)/self.high_water);
  self.max_drawdown=self.max_drawdown.max(dd);
  if self.max_drawdown>costs.max_drawdown{self.halted=true;self.pending=None;}
  self.last_decision=Some(self.decisions.last().filter(|d|d["ts"]==c.ts).cloned().unwrap_or_else(||json!({"ts":c.ts,"action":if signal&&!self.halted{"propose_buy"}else{"wait"},"strategy":STRATEGIES[chosen],"reason":if self.halted{"Drawdownlimiet bereikt"}else if !allow_entry{"Nieuwe entries gepauzeerd of data niet actueel"}else if a.laya&&mode=="neural"&&laya.is_none(){"Laya-regime ontbreekt"}else if self.position.is_some(){"Bestaande positie behoudt haar exitregels"}else if self.cooldown>0{"Cooldown na de vorige positie"}else if signal{"Signaalfilter geslaagd; uitvoering op de volgende candle"}else if chosen==3{"Netwerk kiest niet handelen"}else{"Strategiefilter geeft geen instapsignaal"}})));
  let day=c.ts/86400000;
  if self.last_day!=0&&day!=self.last_day {self.daily.insert(self.last_day,self.last_equity/costs.capital-D::ONE);}
  self.last_day=day;self.last_equity=equity;
  if self.bars%60==0{self.curve.push(json!({"ts":c.ts,"equity":equity.to_string()}));}
  Ok(())
 }
 pub fn report(&self,costs:&Costs)->Value {
  let mut daily=vec![];let mut prev=0.;for (day,cumulative)in &self.daily{let n=f(*cumulative);daily.push(json!({"day":day,"return":n-prev}));prev=n;}
  json!({"net_return":f(self.last_equity/costs.capital-D::ONE),"net_pnl":(self.last_equity-costs.capital).to_string(),"fees":self.fees.to_string(),"execution_costs":self.execution_costs.to_string(),"ai_costs":self.ai_costs.to_string(),"other_costs":self.other_costs.to_string(),"roundtrips":self.trades,"profit_factor":if self.losses>D::ZERO{Some(f(self.wins/self.losses))}else if self.wins>D::ZERO{Some(999.)}else{None},"max_drawdown":self.max_drawdown,"exposure":if self.bars>0{self.exposed as f64/self.bars as f64}else{0.},"valid":!self.halted,"daily_returns":daily,"curve":self.curve,"decisions":self.decisions.iter().rev().take(40).collect::<Vec<_>>(),"activation":self.last_activation,"last_decision":self.last_decision,"position":self.position,"costs":costs,"fill_model":"next candle open + bounded adverse latency; spread/slippage/impact in fill, taker fees separately; conservative stop-first; volume-limited partial fills","feature_version":FEATURES})
 }
}
pub fn decision_inputs(s:&Series,i:usize,entry:Option<D>,age:usize,max_hold:usize,cost_rate:f64,laya:bool)->[f64;INPUTS]{
 let mut raw=s.raw[i];raw[11]=cost_rate;
 if let Some(entry)=entry {raw[8]=1.;raw[9]=f(s.candles[i].close/entry-D::ONE);raw[10]=age as f64/max_hold as f64;}
 if laya{if let Some(v)=s.laya.get(&s.candles[i].ts){raw[12..16].copy_from_slice(v);}}
 raw
}
pub fn entry_signal(s:&Series,i:usize,p:&Parameters,chosen:usize,raw:&[f64;INPUTS])->bool {
 if i<WARMUP{return false;}let c=&s.candles[i];let win=&s.candles[i.saturating_sub(p.lookback)..i];
 match chosen {
  0=>{let ef=tail_ema(&s.candles,i,p.fast);let es=tail_ema(&s.candles,i,p.slow);let pf=tail_ema(&s.candles,i-1,p.fast);let ps=tail_ema(&s.candles,i-1,p.slow);ef>es&&pf<=ps&&raw[3]>0.},
  1=>win.iter().map(|c|c.high).max().is_some_and(|hi|c.close>hi)&&raw[5]>=p.volume_ratio&&raw[7]>=p.min_atr,
  2=>raw[6]<=-p.bb_z&&raw[4]*100.<=p.rsi_low,_=>false
 }
}
fn tail_ema(cs:&[Candle],i:usize,n:usize)->f64 {let start=i.saturating_sub(WARMUP-1);let mut v=f(cs[start].close);for c in &cs[start+1..=i]{v+=2./(n+1)as f64*(f(c.close)-v);}v}
pub fn evaluate(s:&Series,a:&Artifact,costs:&Costs,start:i64,end:i64,mode:&str)->Result<Value>{
 a.validate()?;costs.validate()?;ensure!(start<end,"Evaluatieperiode ongeldig");
 let mut sim=Simulation::new(costs);let mut last=None;
 for (i,_c)in s.candles.iter().enumerate().filter(|(_,c)|c.ts>=start&&c.ts<end){sim.step(s,i,a,costs,mode,true)?;last=Some(i);}
 ensure!(sim.bars>0,"Geen candles binnen evaluatieperiode");
 if let Some(i)=last{let c=&s.candles[i];sim.exit(costs.sell_price(c.close),c.close,costs,c.ts,"split gesloten",true,c.volume);sim.last_equity=sim.cash;sim.daily.insert(c.ts/86400000,sim.cash/costs.capital-D::ONE);}
 let mut report=sim.report(costs);report["start_ms"]=json!(start);report["end_ms"]=json!(end);report["mode"]=json!(mode);Ok(report)
}
#[cfg(test)]
mod tests {
 use super::*;
 fn candles(n:usize)->Vec<Candle>{(0..n).map(|i|{let price=d(100.+(i as f64/12.).sin()*3.);Candle{ts:1_700_000_040_000+i as i64*60000,open:price,high:price+D::ONE,low:price-D::ONE,close:price,volume:D::from(100),closed:true}}).collect()}
 fn artifact()->Artifact {Artifact{schema:1,feature_version:FEATURES.into(),weights:vec![0.;WEIGHTS],parameters:Parameters::default(),normalizer:Normalizer::default(),laya:false,laya_checkpoint:String::new(),laya_runtime:String::new(),instrument:"BTC-EUR".into(),bar:"1m".into(),dataset_hash:String::new(),seed:1,generation:1}}
 #[test]fn no_future_features(){let cs=candles(600);let full=Series::new(cs.clone(),60000).unwrap();let prefix=Series::new(cs[..400].to_vec(),60000).unwrap();assert_eq!(&full.raw[..400],prefix.raw);}
 #[test]fn missing_candle_rejected(){let mut cs=candles(600);cs.remove(300);assert!(Series::new(cs.clone(),60000).is_err());assert!(aggregate(&cs,300000).is_err());}
 #[test]fn normalized_training_only(){let s=Series::new(candles(600),60000).unwrap();let a=s.fit(s.candles[200].ts,s.candles[450].ts).unwrap();let mut t=s.clone();for x in &mut t.raw[450..]{x[0]=99.;}assert_eq!(a.mean,t.fit(s.candles[200].ts,s.candles[450].ts).unwrap().mean);}
 #[test]fn costs_both_sides_basefees_and_rebates(){for base in [false,true]{for fee in [D::new(1,3),D::new(-1,4)]{let costs=Costs{taker_fee:fee,fee_in_base:base,spread_bps:0.,slippage_bps:0.,latency_ms:0,..Costs::default()};let mut a=artifact();a.parameters.stop_bps=300.;a.parameters.target_bps=600.;let s=Series::new(candles(600),60000).unwrap();let mut sim=Simulation::new(&costs);sim.pending=Some(0);sim.step(&s,300,&a,&costs,"cash",true).unwrap();let p=sim.position.clone().unwrap();sim.exit(p.entry,p.entry,&costs,s.candles[301].ts,"test",true,D::from(10000));let expected=if base {p.quantity/(D::ONE+fee)*p.entry-p.debit}else{p.quantity*p.entry*(D::ONE-fee)-p.debit};assert!((sim.cash-costs.capital-expected).abs()<D::new(1,8));assert_eq!(sim.trades,1);}}}
 #[test]fn replay_forward_identical_and_deduplicated(){let s=Series::new(candles(600),60000).unwrap();let a=artifact();let costs=Costs::default();let mut a1=Simulation::new(&costs);let mut a2=Simulation::new(&costs);for i in 200..500{a1.step(&s,i,&a,&costs,"mean_reversion",true).unwrap();if i==350{a2=serde_json::from_value(serde_json::to_value(&a2).unwrap()).unwrap();}a2.step(&s,i,&a,&costs,"mean_reversion",true).unwrap();}assert_eq!(a1.report(&costs),a2.report(&costs));assert!(a2.step(&s,499,&a,&costs,"cash",true).is_err());}
 #[test]fn laya_failure_blocks_only_entries(){let mut a=artifact();a.laya=true;a.laya_checkpoint="a".repeat(40);let s=Series::new(candles(600),60000).unwrap();let mut sim=Simulation::new(&Costs::default());a.weights[338]=4.;for i in 200..500{sim.step(&s,i,&a,&Costs::default(),"neural",true).unwrap();}assert_eq!(sim.trades,0);assert!(sim.position.is_none());}
 #[test]fn aggregation_closed_complete(){let cs=candles(600);let out=aggregate(&cs,300000).unwrap();assert_eq!(out[0].ts%300000,0);assert_eq!(out[0].volume,D::from(500));}
 #[test]fn bounded_forward_window_has_identical_features(){let cs=candles(1000);let all=Series::new(cs.clone(),60000).unwrap();let tail=Series::new(cs[500..].to_vec(),60000).unwrap();assert_eq!(&all.raw[700..],&tail.raw[200..]);}
 #[test]fn frozen_exit_and_partial_inventory(){let s=Series::new(candles(600),60000).unwrap();let costs=Costs::default();let mut a=artifact();a.parameters.stop_bps=300.;a.parameters.target_bps=600.;let mut sim=Simulation::new(&costs);sim.pending=Some(2);sim.step(&s,300,&a,&costs,"cash",true).unwrap();let p=sim.position.clone().unwrap();sim.exit(costs.sell_price(p.entry),p.entry,&costs,s.candles[301].ts,"partial",false,D::from(1));let remaining=sim.position.clone().unwrap();assert!(remaining.quantity<p.quantity);assert_eq!(remaining.stop,p.stop);assert_eq!(remaining.target,p.target);assert_eq!(remaining.strategy,2);assert_eq!(sim.trades,0);}
 #[test]fn protective_exit_survives_entry_block(){let mut cs=candles(600);let costs=Costs::default();let mut a=artifact();a.parameters.stop_bps=300.;a.parameters.target_bps=600.;let mut sim=Simulation::new(&costs);sim.pending=Some(2);let s=Series::new(cs.clone(),60000).unwrap();sim.step(&s,300,&a,&costs,"cash",true).unwrap();assert!(sim.position.is_some());cs[301].low=D::from(70);let s=Series::new(cs,60000).unwrap();sim.step(&s,301,&a,&costs,"neural",false).unwrap();assert!(sim.position.is_none());assert_eq!(sim.trades,1);}
}
