//! Local research control plane. Immutable evidence; no training-to-exchange path.
use crate::{App,CandleQuery,candle_result,model::{Candle,now_ms},neuro::{self,Artifact,Costs,Simulation,Series},neuro_data,studio::reply};
use anyhow::{Result,ensure};
use axum::{Router,Json,extract::{State,Path},response::Response,routing::{get,post}};
use serde::Deserialize;
use serde_json::{Value,json};
use sqlx::Row;
use std::{path::PathBuf,collections::BTreeMap};
pub const LAYA_PIN:&str="7b928d828b7b0e022f929d9bd2e44165aa270148";
fn laya_url()->String{std::env::var("LAYA_URL").unwrap_or_else(|_|"http://host.docker.internal:8092".into())}
pub fn root()->PathBuf{PathBuf::from(std::env::var("TRAINING_DIR").unwrap_or_else(|_|"/training".into()))}
fn uid()->String {let mut bytes=[0u8;16];use ring::rand::{SecureRandom,SystemRandom};SystemRandom::new().fill(&mut bytes).expect("OS randomness");bytes.iter().map(|b|format!("{b:02x}")).collect()}
fn identifier(s:&str)->Result<()> {ensure!(s.len()<=64&&!s.is_empty()&&s.bytes().all(|b|b.is_ascii_hexdigit()),"Ongeldig onderzoeks-id");Ok(())}
async fn write_json(path:PathBuf,value:&Value)->Result<()> {let data=serde_json::to_vec(value)?;if let Some(p)=path.parent(){tokio::fs::create_dir_all(p).await?;}let tmp=path.with_extension("tmp");tokio::fs::write(&tmp,&data).await?;tokio::fs::rename(tmp,path).await?;Ok(())}
async fn read_json(path:PathBuf,max:u64)->Result<Value> {ensure!(tokio::fs::metadata(&path).await?.len()<=max,"Onderzoeksbestand te groot");Ok(serde_json::from_slice(&tokio::fs::read(path).await?)?)}
async fn audit(a:&App,kind:&str,id:&str,detail:Value)->Result<()> {sqlx::query("INSERT INTO training.audit(kind,subject,detail,at_ms) VALUES($1,$2,$3,$4)").bind(kind).bind(id).bind(detail).bind(now_ms()).execute(&a.db).await?;Ok(())}
pub fn router()->Router<App>{Router::new()
 .merge(crate::training_transfer::router())
 .route("/api/training",get(overview))
 .route("/api/training/datasets",post(import_dataset))
 .route("/api/training/datasets/{id}/resume",post(resume_dataset))
 .route("/api/training/campaigns",post(create_campaign))
 .route("/api/training/campaigns/{id}/{action}",post(control))
 .route("/api/training/campaigns/{id}/snapshots/{name}",get(snapshot))
 .route("/api/training/artifacts/{hash}",get(artifact))
 .route("/api/training/artifacts/{hash}/qualify",post(qualify))
 .route("/api/training/artifacts/{hash}/forward",post(start_forward))
 .route("/api/training/forward/{id}/{action}",post(forward_control))
 .route("/api/training/artifacts/{hash}/release",post(release))
}
async fn overview(State(a):State<App>)->Response {reply(async{
 let datasets=sqlx::query("SELECT * FROM training.datasets ORDER BY updated_ms DESC").fetch_all(&a.db).await?.iter().map(|r|json!({"id":r.get::<String,_>("id"),"instrument":r.get::<String,_>("instrument"),"start_ms":r.get::<i64,_>("start_ms"),"end_ms":r.get::<i64,_>("end_ms"),"status":r.get::<String,_>("status"),"manifest":r.get::<Value,_>("manifest")})).collect::<Vec<_>>();
 let campaigns=sqlx::query("SELECT * FROM training.campaigns ORDER BY created_ms DESC LIMIT 50").fetch_all(&a.db).await?.iter().map(|r|json!({"id":r.get::<String,_>("id"),"config":r.get::<Value,_>("config"),"status":r.get::<String,_>("status"),"desired":r.get::<String,_>("desired"),"progress":r.get::<Value,_>("progress")})).collect::<Vec<_>>();
 let artifacts=sqlx::query("SELECT hash,campaign_id,instrument,bar,summary,created_ms FROM training.artifacts ORDER BY created_ms DESC LIMIT 100").fetch_all(&a.db).await?.iter().map(|r|json!({"hash":r.get::<String,_>("hash"),"campaign_id":r.get::<String,_>("campaign_id"),"instrument":r.get::<String,_>("instrument"),"bar":r.get::<String,_>("bar"),"summary":r.get::<Value,_>("summary"),"created_ms":r.get::<i64,_>("created_ms")})).collect::<Vec<_>>();
 let qualifications=sqlx::query("SELECT artifact_hash,status,report,start_ms,end_ms FROM training.holdouts ORDER BY created_ms DESC LIMIT 100").fetch_all(&a.db).await?.iter().map(|r|json!({"hash":r.get::<String,_>("artifact_hash"),"status":r.get::<String,_>("status"),"report":r.get::<Option<Value>,_>("report"),"start_ms":r.get::<i64,_>("start_ms"),"end_ms":r.get::<i64,_>("end_ms")})).collect::<Vec<_>>();
 let forwards=sqlx::query("SELECT id,artifact_hash,status,report,started_ms,updated_ms FROM training.forward_runs ORDER BY started_ms DESC LIMIT 100").fetch_all(&a.db).await?.iter().map(|r|json!({"id":r.get::<String,_>("id"),"hash":r.get::<String,_>("artifact_hash"),"status":r.get::<String,_>("status"),"report":r.get::<Value,_>("report"),"started_ms":r.get::<i64,_>("started_ms"),"updated_ms":r.get::<i64,_>("updated_ms")})).collect::<Vec<_>>();
 let worker=read_json(root().join("heartbeat.json"),4096).await.unwrap_or(json!({"status":"offline"}));
 let laya=match a.http.get(format!("{}/health",laya_url())).timeout(std::time::Duration::from_secs(2)).send().await {Ok(r)=>r.json::<Value>().await.unwrap_or(json!({"ready":false})),Err(_)=>json!({"ready":false,"reason":"Lokale Laya-service niet bereikbaar"})};
 Ok(json!({"datasets":datasets,"campaigns":campaigns,"artifacts":artifacts,"qualifications":qualifications,"forwards":forwards,"worker":worker,"laya":laya,"feature_labels":neuro::LABELS,"network":{"inputs":16,"hidden":16,"outputs":4,"weights":neuro::WEIGHTS},"defaults":{"population":64,"seeds":[17,41,73],"generations":200,"patience":30,"workers":2,"memory_mb":4096,"daily_hours":8},"release_ready":false,"release_reason":"Native botvrijgave vereist historische kwalificatie, 30 dagen en 100 forward-roundtrips, actuele accountlimieten en voor Live echte Demo-acceptatie. Handmatige vrijgave geeft geen bottoestemming."}))
 }.await)}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetRequest {instrument:String,start_ms:i64,end_ms:i64}
async fn import_dataset(State(a):State<App>,Json(r):Json<DatasetRequest>)->Response{reply(async{
 ensure!(r.start_ms>0&&r.start_ms%60000==0&&r.end_ms%60000==0&&r.end_ms<=now_ms()/60000*60000&&r.end_ms>r.start_ms&&(r.end_ms-r.start_ms)/60000<=2_000_000,"Selecteer gesloten datums, maximaal 2 miljoen minuten; geen stille afkapgrens");
 ensure!(a.market.read().await.catalogue.iter().any(|m|m["instId"]==r.instrument&&(m["quoteCcy"]=="EUR"||m["quoteCcy"]=="USDC")),"Kies een EUR/USDC-spotmarkt uit de publieke catalogus");
 ensure!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM training.datasets WHERE status='IMPORTING'").fetch_one(&a.db).await?<2,"Maximaal twee datasetimports tegelijk");
 let id=uid();sqlx::query("INSERT INTO training.datasets(id,instrument,start_ms,end_ms,status,updated_ms) VALUES($1,$2,$3,$4,'IMPORTING',$5)").bind(&id).bind(&r.instrument).bind(r.start_ms).bind(r.end_ms).bind(now_ms()).execute(&a.db).await?;
 audit(&a,"DATASET_IMPORT",&id,json!({"instrument":r.instrument,"start_ms":r.start_ms,"end_ms":r.end_ms})).await?;
 launch_import(a,id.clone());Ok(json!({"id":id,"status":"IMPORTING"}))
 }.await)}
async fn resume_dataset(State(a):State<App>,Path(id):Path<String>)->Response{reply(async{identifier(&id)?;let status:String=sqlx::query_scalar("SELECT status FROM training.datasets WHERE id=$1").bind(&id).fetch_one(&a.db).await?;ensure!(["ERROR","INTERRUPTED"].contains(&status.as_str()),"Dataset is niet onderbroken");sqlx::query("UPDATE training.datasets SET status='IMPORTING',updated_ms=$2 WHERE id=$1").bind(&id).bind(now_ms()).execute(&a.db).await?;launch_import(a,id.clone());Ok(json!({"id":id,"status":"IMPORTING"}))}.await)}
fn launch_import(a:App,id:String){tokio::spawn(async move{if let Err(e)=do_import(&a,&id).await{let _=sqlx::query("UPDATE training.datasets SET status='ERROR',manifest=manifest||$2,updated_ms=$3 WHERE id=$1").bind(&id).bind(json!({"error":e.to_string()})).bind(now_ms()).execute(&a.db).await;}});}
async fn do_import(a:&App,id:&str)->Result<()> {
 let row=sqlx::query("SELECT instrument,start_ms,end_ms,manifest FROM training.datasets WHERE id=$1").bind(id).fetch_one(&a.db).await?;
 let instrument:String=row.get("instrument");let start:i64=row.get("start_ms");let end:i64=row.get("end_ms");let manifest:Value=row.get("manifest");let mut after=manifest["next_after"].as_i64().unwrap_or(end);
 let dir=root().join("imports").join(id);tokio::fs::create_dir_all(&dir).await?;
 let mut parts:Vec<String>=manifest["parts"].as_array().map(|v|v.iter().filter_map(|x|x.as_str().map(str::to_owned)).collect()).unwrap_or_default();let mut pages=manifest["pages"].as_u64().unwrap_or(0);
 loop {
  let data=candle_result(a,&CandleQuery{instrument:instrument.clone(),bar:"1m".into(),after:Some(after)}).await?;
  let mut cs:Vec<Candle>=serde_json::from_value(data["candles"].clone())?;cs.retain(|c|c.closed&&c.ts<after&&c.ts>=start&&c.ts<end);
  if cs.is_empty(){break;}
  let first=cs[0].ts;ensure!(first<after,"Importcursor maakte geen voortgang");
  let name=format!("{first}.parquet");tokio::fs::write(dir.join(&name),neuro_data::encode(&cs)?).await?;parts.push(name);after=first;pages+=1;
  let progress=json!({"parts":parts,"next_after":after,"pages":pages,"progress":((end-after)as f64/(end-start)as f64).clamp(0.,1.),"source":"OKX EEA /market/history-candles","bar":"1m","closed_only":true,"error":null});
  sqlx::query("UPDATE training.datasets SET manifest=$2,updated_ms=$3 WHERE id=$1").bind(id).bind(progress).bind(now_ms()).execute(&a.db).await?;
  if after<=start{break;}
 }
 let target=root().join("datasets").join(format!("{id}.parquet"));tokio::fs::create_dir_all(target.parent().unwrap()).await?;
 static ASSEMBLY:std::sync::OnceLock<tokio::sync::Semaphore>=std::sync::OnceLock::new();
 let _assembly=ASSEMBLY.get_or_init(||tokio::sync::Semaphore::new(1)).acquire().await?;
 let (count,first,last,gaps,hash)=tokio::task::spawn_blocking(move||->Result<_>{let mut all=BTreeMap::new();for name in parts{for c in neuro_data::decode(&dir.join(name))?{if let Some(previous)=all.insert(c.ts,c.clone()){ensure!(serde_json::to_value(previous)?==serde_json::to_value(c)?,"Conflicterende historische candles");}}}let cs:Vec<_>=all.into_values().collect();ensure!(!cs.is_empty(),"Geen historie beschikbaar in dit datumbereik");let first=cs[0].ts;let last=cs.last().unwrap().ts;let gaps:Vec<_>=cs.windows(2).filter(|w|w[1].ts-w[0].ts!=60000).map(|w|json!({"after":w[0].ts,"before":w[1].ts,"missing":(w[1].ts-w[0].ts)/60000-1})).collect();let bytes=neuro_data::encode(&cs)?;let hash=neuro_data::hash(&bytes);std::fs::write(target.with_extension("tmp"),bytes)?;std::fs::rename(target.with_extension("tmp"),target)?;Ok((cs.len(),first,last,gaps,hash))}).await??;
 let complete=gaps.is_empty()&&first==start&&last+60000==end;
 let manifest=json!({"rows":count,"first_ms":first,"last_ms":last,"requested_start_ms":start,"requested_end_ms":end,"coverage":count as f64/((end-start)/60000)as f64,"gaps":gaps,"hash":hash,"source":"OKX EEA closed 1m; 5m/15m aggregated from complete aligned groups","pages":pages,"progress":1.0,"complete":complete,"path":format!("/training/datasets/{id}.parquet"),"version":1,"created_ms":now_ms()});
 write_json(root().join("datasets").join(format!("{id}.json")),&manifest).await?;
 sqlx::query("UPDATE training.datasets SET status=$2,manifest=$3,updated_ms=$4 WHERE id=$1").bind(id).bind(if complete{"READY"}else{"INCOMPLETE"}).bind(&manifest).bind(now_ms()).execute(&a.db).await?;audit(a,"DATASET_SEALED",id,manifest).await?;Ok(())
}
fn def_population()->u32{64}fn def_gen()->u32{200}fn def_hours()->f64{8.}fn def_workers()->u32{2}fn def_mem()->u32{4096}fn def_laya()->bool{true}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignRequest {name:String,dataset_ids:Vec<String>,bars:Vec<String>,#[serde(default="def_population")]population:u32,#[serde(default="def_gen")]generations:u32,#[serde(default="def_hours")]daily_hours:f64,#[serde(default="def_workers")]workers:u32,#[serde(default="def_mem")]memory_mb:u32,#[serde(default="def_laya")]laya:bool,costs:Costs}
async fn create_campaign(State(a):State<App>,Json(r):Json<CampaignRequest>)->Response {reply(async{
 ensure!(!r.name.trim().is_empty()&&r.name.len()<=100&&(1..=8).contains(&r.dataset_ids.len())&&(1..=3).contains(&r.bars.len())&&r.bars.iter().all(|b|["1m","5m","15m"].contains(&b.as_str())),"Campagnenaam/datasets/timeframes ongeldig");
 ensure!((8..=64).contains(&r.population)&&(1..=200).contains(&r.generations)&&(1..=2).contains(&r.workers)&&r.memory_mb==4096&&r.daily_hours.is_finite()&&(0.01..=8.).contains(&r.daily_hours),"Resourcegrenzen overschreden");r.costs.validate()?;
 ensure!(r.bars.iter().collect::<std::collections::BTreeSet<_>>().len()==r.bars.len(),"Dubbele timeframes in campagne");
 let mut datasets=vec![];let mut unique=std::collections::BTreeSet::new();
 for id in &r.dataset_ids {identifier(id)?;let row=sqlx::query("SELECT instrument,status,manifest FROM training.datasets WHERE id=$1").bind(id).fetch_one(&a.db).await?;ensure!(row.get::<String,_>("status")=="READY","Dataset niet volledig: {id}");let m:Value=row.get("manifest");let start=m["first_ms"].as_i64().unwrap_or(0);let end=m["last_ms"].as_i64().unwrap_or(0)+60000;ensure!(end-start>=365*86400000,"Minimaal twaalf maanden aaneengesloten historie vereist");let instrument:String=row.get("instrument");ensure!(unique.insert(instrument.clone()),"Dubbele markt in campagne");let metadata=a.market.read().await.catalogue.iter().find(|v|v["instId"]==instrument).cloned().ok_or_else(||anyhow::anyhow!("Marktmetadata ontbreekt"))?;
  let mut costs=r.costs.clone();costs.lot=metadata["lotSz"].as_str().unwrap_or("").parse()?;costs.tick=metadata["tickSz"].as_str().unwrap_or("").parse()?;costs.minimum=metadata["minSz"].as_str().unwrap_or("").parse()?;
  datasets.push(json!({"id":id,"instrument":instrument,"path":m["path"],"hash":m["hash"],"start_ms":start,"end_ms":end,"holdout_start_ms":end-60*86400000,"costs":costs}));
 }
 let laya:Value=if r.laya{a.http.get(format!("{}/health",laya_url())).timeout(std::time::Duration::from_secs(2)).send().await?.json().await?}else{json!({})};if r.laya{ensure!(laya["ready"]==true&&laya["checkpoint"]==LAYA_PIN,"Vastgezet Laya-model is niet gereed");}
 let id=uid();let config=json!({"schema":1,"id":id,"name":r.name,"datasets":datasets,"bars":r.bars,"population":r.population,"generations":r.generations,"daily_hours":r.daily_hours,"workers":r.workers,"memory_mb":r.memory_mb,"laya":r.laya,"laya_checkpoint":LAYA_PIN,"laya_runtime":if r.laya{laya["runtime"].clone()}else{json!("")},"seeds":[17,41,73],"patience":30,"feature_version":neuro::FEATURES,"qualification":{"min_trades":200,"profit_factor":1.2,"max_drawdown":0.05,"cost_stress":1.5,"forward_days":30,"forward_trades":100,"bootstrap_seed":91271,"bootstrap_repetitions":2000,"holm_alpha":0.05},"created_ms":now_ms()});
 let bytes=serde_json::to_vec(&config)?;let hash=neuro_data::hash(&bytes);let dir=root().join("campaigns").join(&id);tokio::fs::create_dir_all(&dir).await?;tokio::fs::write(dir.join("request.json"),bytes).await?;
 sqlx::query("INSERT INTO training.campaigns(id,config,config_hash,desired,status,created_ms,updated_ms) VALUES($1,$2,$3,'RUNNING','QUEUED',$4,$4)").bind(&id).bind(&config).bind(hash).bind(now_ms()).execute(&a.db).await?;
 write_json(dir.join("control.json"),&json!({"desired":"RUNNING"})).await?;audit(&a,"CAMPAIGN_CREATED",&id,config).await?;Ok(json!({"id":id,"status":"QUEUED"}))
 }.await)}
async fn control(State(a):State<App>,Path((id,action)):Path<(String,String)>)->Response {reply(async{
 identifier(&id)?;ensure!(["pause","resume","stop"].contains(&action.as_str()),"Kies pauzeren, hervatten of stoppen");let desired=match action.as_str(){"resume"=>"RUNNING","pause"=>"PAUSED",_=>"STOPPED"};
 ensure!(sqlx::query("UPDATE training.campaigns SET desired=$2,updated_ms=$3 WHERE id=$1").bind(&id).bind(desired).bind(now_ms()).execute(&a.db).await?.rows_affected()==1,"Campagne bestaat niet");write_json(root().join("campaigns").join(&id).join("control.json"),&json!({"desired":desired})).await?;audit(&a,"CAMPAIGN_CONTROL",&id,json!({"desired":desired})).await?;Ok(json!({"id":id,"desired":desired}))
 }.await)}
async fn artifact(State(a):State<App>,Path(hash):Path<String>)->Response {reply(async{identifier(&hash)?;let row=sqlx::query("SELECT artifact,summary FROM training.artifacts WHERE hash=$1").bind(&hash).fetch_one(&a.db).await?;Ok(json!({"hash":hash,"artifact":row.get::<Value,_>("artifact"),"summary":row.get::<Value,_>("summary")}))}.await)}
async fn snapshot(State(_a):State<App>,Path((id,name)):Path<(String,String)>)->Response {reply(async{identifier(&id)?;ensure!(name.len()<=160&&name.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'),"Snapshotnaam ongeldig");read_json(root().join("campaigns").join(id).join("generations").join(format!("{name}.json")),64000).await}.await)}
async fn artifact_row(a:&App,hash:&str)->Result<(Artifact,Value,Value)> {identifier(hash)?;let row=sqlx::query("SELECT a.artifact,a.summary,c.config FROM training.artifacts a JOIN training.campaigns c ON c.id=a.campaign_id WHERE a.hash=$1").bind(hash).fetch_one(&a.db).await?;let artifact:Artifact=serde_json::from_value(row.get("artifact"))?;artifact.validate()?;Ok((artifact,row.get("summary"),row.get("config")))}
async fn qualify(State(a):State<App>,Path(hash):Path<String>)->Response {reply(async{
 let (artifact,_,config)=artifact_row(&a,&hash).await?;let ds=config["datasets"].as_array().unwrap().iter().find(|v|v["instrument"]==artifact.instrument).unwrap();let start=ds["holdout_start_ms"].as_i64().unwrap();let end=ds["end_ms"].as_i64().unwrap();
 let mut tx=a.db.begin().await?;sqlx::query("SELECT pg_advisory_xact_lock(775902)").execute(&mut *tx).await?;
 let family=sqlx::query("SELECT hash,artifact FROM training.artifacts WHERE campaign_id=$1").bind(config["id"].as_str().unwrap()).fetch_all(&mut *tx).await?;
 ensure!(family.len()==config["datasets"].as_array().unwrap().len()*config["bars"].as_array().unwrap().len(),"Wacht tot alle campagnekandidaten zijn vastgezet; de kwalificatiefamilie blijft vast");
 for member in &family {let art:Artifact=serde_json::from_value(member.get("artifact"))?;let ds=config["datasets"].as_array().unwrap().iter().find(|v|v["instrument"]==art.instrument).unwrap();let start=ds["holdout_start_ms"].as_i64().unwrap();let end=ds["end_ms"].as_i64().unwrap();let overlap:i64=sqlx::query_scalar("SELECT count(*) FROM training.holdouts WHERE instrument=$1 AND start_ms<$3 AND end_ms>$2").bind(&art.instrument).bind(start).bind(end).fetch_one(&mut *tx).await?;ensure!(overlap==0,"Deze eindtest of overlappende data is al bekeken. Nieuwe ongebruikte data vereist.");}
 for member in &family {let art:Artifact=serde_json::from_value(member.get("artifact"))?;let h:String=member.get("hash");let ds=config["datasets"].as_array().unwrap().iter().find(|v|v["instrument"]==art.instrument).unwrap();sqlx::query("INSERT INTO training.holdouts(instrument,start_ms,end_ms,artifact_hash,campaign_id,status,created_ms) VALUES($1,$2,$3,$4,$5,'QUEUED',$6)").bind(&art.instrument).bind(ds["holdout_start_ms"].as_i64().unwrap()).bind(ds["end_ms"].as_i64().unwrap()).bind(&h).bind(config["id"].as_str().unwrap()).bind(now_ms()).execute(&mut *tx).await?;}
 tx.commit().await?;
 for member in family {let h:String=member.get("hash");let art:Value=member.get("artifact");let dir=root().join("qualification").join(&h);write_json(dir.join("request.json"),&json!({"artifact":art,"config":config,"hash":h})).await?;}
 audit(&a,"HOLDOUT_FAMILY_CONSUMED",&hash,json!({"start_ms":start,"end_ms":end})).await?;Ok(json!({"status":"QUEUED","hash":hash}))
 }.await)}
async fn start_forward(State(a):State<App>,Path(hash):Path<String>)->Response {reply(async{
 let (artifact,_,config)=artifact_row(&a,&hash).await?;let historical:Option<String>=sqlx::query_scalar("SELECT status FROM training.holdouts WHERE artifact_hash=$1").bind(&hash).fetch_optional(&a.db).await?;ensure!(historical.as_deref()==Some("PASSED"),"Historische kwalificatie moet eerst volledig slagen");
 ensure!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM training.forward_runs WHERE artifact_hash=$1").bind(&hash).fetch_one(&a.db).await?==0,"Forwardbewijs wordt niet gereset. Hervat de bestaande run.");
 let ds=config["datasets"].as_array().unwrap().iter().find(|v|v["instrument"]==artifact.instrument).unwrap();let costs:Costs=serde_json::from_value(ds["costs"].clone())?;let id=uid();let state=json!({"simulation":Simulation::new(&costs),"costs":costs,"candles":[],"started_ms":now_ms()});
 sqlx::query("INSERT INTO training.forward_runs(id,artifact_hash,status,state,started_ms,updated_ms) VALUES($1,$2,'RUNNING',$3,$4,$4)").bind(&id).bind(&hash).bind(state).bind(now_ms()).execute(&a.db).await?;audit(&a,"FORWARD_START",&id,json!({"hash":hash,"instrument":artifact.instrument,"bar":artifact.bar})).await?;Ok(json!({"id":id,"status":"RUNNING"}))
 }.await)}
async fn forward_control(State(a):State<App>,Path((id,action)):Path<(String,String)>)->Response {reply(async{identifier(&id)?;ensure!(["pause","resume"].contains(&action.as_str()),"Kies pauzeren of hervatten");let row=sqlx::query("SELECT state,status FROM training.forward_runs WHERE id=$1").bind(&id).fetch_one(&a.db).await?;let state:Value=row.get("state");ensure!(action!="resume"||state["simulation"]["halted"]!=true,"Risicostop: nieuwe kwalificatie vereist; verlieshistorie blijft behouden");let status=if action=="pause"{"PAUSED"}else{"RUNNING"};sqlx::query("UPDATE training.forward_runs SET status=$2 WHERE id=$1").bind(&id).bind(status).execute(&a.db).await?;audit(&a,"FORWARD_CONTROL",&id,json!({"status":status})).await?;Ok(json!({"id":id,"status":status}))}.await)}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseRequest {account_uid:String,budget:String,confirmed:bool}
async fn release(State(a):State<App>,Path(hash):Path<String>,Json(r):Json<ReleaseRequest>)->Response {reply(async{ensure!(r.confirmed&&!r.account_uid.is_empty()&&r.budget.parse::<rust_decimal::Decimal>()?>rust_decimal::Decimal::ZERO,"Bevestig je eigen account en botbudget");let _=artifact_row(&a,&hash).await?;ensure!(false,"Gebruik de exacte native-release met omgeving en alle bevestigde limieten. Historisch, forward- en echt Demo-bewijs blijven verplicht.");Ok(json!({}))}.await)}
pub async fn run(a:App) {
 if tokio::fs::create_dir_all(root()).await.is_err(){return;}
 let Ok(mut lease)=a.db.acquire().await else{return;};
 let Ok(true)=sqlx::query_scalar::<_,bool>("SELECT pg_try_advisory_lock(775903)").fetch_one(&mut *lease).await else{return;};
 let _=sqlx::query("UPDATE training.datasets SET status='INTERRUPTED' WHERE status='IMPORTING'").execute(&a.db).await;
 tokio::spawn(crate::training_feed::run(a.clone()));
 tokio::spawn(crate::training_feed::record(a.clone()));
 loop {if sqlx::query("SELECT 1").execute(&mut *lease).await.is_err(){break;}if let Err(e)=sync_results(&a).await{tracing::warn!(error=%e,"Training result sync");}if let Err(e)=forward_tick(&a).await{tracing::warn!(error=%e,"Forward paper tick");}tokio::time::sleep(std::time::Duration::from_secs(3)).await;}
}
async fn sync_results(a:&App)->Result<()> {
 let rows=sqlx::query("SELECT id FROM training.campaigns").fetch_all(&a.db).await?;
 for row in rows {let id:String=row.get("id");let dir=root().join("campaigns").join(&id);let Ok(progress)=read_json(dir.join("progress.json"),8_000_000).await else{continue};
  let status=progress["status"].as_str().unwrap_or("UNKNOWN");sqlx::query("UPDATE training.campaigns SET status=$2,progress=$3,updated_ms=$4 WHERE id=$1").bind(&id).bind(status).bind(&progress).bind(now_ms()).execute(&a.db).await?;
  if let Some(xs)=progress["artifacts"].as_array(){for item in xs{let hash=item["hash"].as_str().unwrap_or("");identifier(hash)?;let path=dir.join("artifacts").join(format!("{hash}.json"));let bytes=tokio::fs::read(&path).await?;ensure!(bytes.len()<64_000&&neuro_data::hash(&bytes)==hash,"Artifacthash wijkt af");let artifact:Artifact=serde_json::from_slice(&bytes)?;artifact.validate()?;
   sqlx::query("INSERT INTO training.artifacts(hash,campaign_id,instrument,bar,artifact,summary,created_ms) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING").bind(hash).bind(&id).bind(&artifact.instrument).bind(&artifact.bar).bind(serde_json::to_value(&artifact)?).bind(&item["summary"]).bind(now_ms()).execute(&a.db).await?;
  }}
 }
 let qs=sqlx::query("SELECT artifact_hash,campaign_id FROM training.holdouts WHERE status='QUEUED'").fetch_all(&a.db).await?;
 for r in qs{let hash:String=r.get("artifact_hash");if let Ok(report)=read_json(root().join("qualification").join(&hash).join("report.json"),8_000_000).await{if report["family_complete"]!=true{continue;}let status=if report["passed"]==true{"PASSED"}else{"FAILED"};sqlx::query("UPDATE training.holdouts SET status=$2,report=$3 WHERE artifact_hash=$1").bind(&hash).bind(status).bind(&report).execute(&a.db).await?;audit(a,"QUALIFICATION_RESULT",&hash,report).await?;}}
 Ok(())
}
async fn forward_tick(a:&App)->Result<()> {
 let rows=sqlx::query("SELECT f.id,f.state,f.status,f.started_ms,a.artifact FROM training.forward_runs f JOIN training.artifacts a ON a.hash=f.artifact_hash WHERE f.status IN ('RUNNING','PAUSED','RISK_STOP')").fetch_all(&a.db).await?;
 for row in rows{let id:String=row.get("id");let artifact:Artifact=serde_json::from_value(row.get("artifact"))?;let mut state:Value=row.get("state");let costs:Costs=serde_json::from_value(state["costs"].clone())?;let mut sim:Simulation=serde_json::from_value(state["simulation"].clone())?;
  let result=crate::training_feed::recent(a,&artifact.instrument).await;
  let mut reason=None;
  if let Ok(data)=result {if data["gaps"]!=0||data["fresh"]!=true{reason=Some("Marktdata verouderd of datagat".to_string());}else{
   let cs:Vec<Candle>=serde_json::from_value(data["candles"].clone())?;let previous:Vec<Candle>=serde_json::from_value(state["candles"].clone()).unwrap_or_default();let mut all:BTreeMap<_,_>=previous.into_iter().map(|c|(c.ts,c)).collect();for c in cs.into_iter().filter(|c|c.closed){all.insert(c.ts,c);}let minute:Vec<_>=all.into_values().rev().take(3200).collect::<Vec<_>>().into_iter().rev().collect();state["candles"]=serde_json::to_value(&minute)?;
   let bars=neuro::aggregate(&minute,crate::model::interval(&artifact.bar).unwrap())?;
   if bars.len()>neuro::WARMUP {let mut s=Series::new(bars,crate::model::interval(&artifact.bar).unwrap())?;let start_index=if sim.last_ms==0{s.candles.len()-1}else{s.candles.iter().position(|c|c.ts>sim.last_ms).unwrap_or(s.candles.len())};
    for i in start_index..s.candles.len(){let c=&s.candles[i];if c.ts+s.step>now_ms(){continue;}if sim.last_ms!=0&&c.ts!=sim.last_ms+s.step {reason=Some("Forward datagat: hervatten vereist dat herstel volledig is".into());break;}
     let fresh=c.ts+s.step+15000>=now_ms();if artifact.laya&&fresh{match laya_values(a,&s.raw[i],c.ts,&artifact.instrument,&artifact.bar,&artifact.laya_runtime).await{Ok(values)=>{s.laya.insert(c.ts,values);},Err(e)=>{reason=Some(format!("Laya-uitval: {e}"));}}}
     sim.step(&s,i,&artifact,&costs,"neural",fresh&&reason.is_none()&&row.get::<String,_>("status")=="RUNNING")?;
    }
   }else{reason=Some(format!("Warm-up: {} / {} gesloten {}-candles",bars.len(),neuro::WARMUP,artifact.bar));}
  }}else{reason=Some("Geen actuele candles".into());}
  let elapsed=now_ms()-row.get::<i64,_>("started_ms");let observed=sim.bars as i64*crate::model::interval(&artifact.bar).unwrap();let mut report=sim.report(&costs);report["elapsed_days"]=json!(observed as f64/86400000.);report["calendar_days"]=json!(elapsed as f64/86400000.);report["reason"]=json!(reason);report["passed"]=json!(elapsed>=30*86400000&&observed>=30*86400000&&sim.trades>=100&&report["net_return"].as_f64().unwrap_or(0.)>0.&&sim.max_drawdown<=0.05&&!sim.halted);report["scope"]=json!("Live closed candles; simulated fills. Native order protection not accepted.");state["simulation"]=serde_json::to_value(&sim)?;
  let mut tx=a.db.begin().await?;let status=if sim.halted{"RISK_STOP".to_string()}else{row.get::<String,_>("status")};sqlx::query("UPDATE training.forward_runs SET state=$2,report=$3,status=$4,updated_ms=$5 WHERE id=$1 AND status=$6").bind(&id).bind(state).bind(report).bind(status).bind(now_ms()).bind(row.get::<String,_>("status")).execute(&mut *tx).await?;tx.commit().await?;
 }
 Ok(())
}
pub(crate) async fn laya_values(a:&App,features:&[f64;16],ts:i64,instrument:&str,bar:&str,runtime:&str)->Result<[f64;4]> {
 let value:Value=a.http.post(format!("{}/regime",laya_url())).timeout(std::time::Duration::from_secs(2)).json(&json!({"rows":[{"ts":ts,"features":features}],"instrument":instrument,"bar":bar,"checkpoint":LAYA_PIN})).send().await?.error_for_status()?.json().await?;ensure!(value["checkpoint"]==LAYA_PIN&&value["runtime"]==runtime,"Laya-modelpin wijkt af");let vals:[f64;4]=serde_json::from_value(value["rows"][0]["values"].clone())?;ensure!(vals.iter().all(|v|v.is_finite()&&(0.0..=1.).contains(v)),"Laya-resultaat ongeldig");Ok(vals)
}
