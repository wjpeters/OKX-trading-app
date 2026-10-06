//! Portable network snapshots. Imports are research copies, never trading permissions.
use crate::{App, model::now_ms, neuro::{Artifact,WEIGHTS}, neuro_data, studio::reply, training};
use anyhow::{Result,ensure};
use axum::{Router,Json,extract::{State,Path,Query,DefaultBodyLimit},response::Response,routing::{get,post}};
use serde::{Serialize,Deserialize};
use serde_json::{Value,json};
use sqlx::Row;

const FORMAT:&str="okx-neuro-network";
const MAX_BYTES:usize=128_000;
const FIELDS:[&str;13]=["schema","feature_version","weights","parameters","normalizer","laya","laya_checkpoint","laya_runtime","instrument","bar","dataset_hash","seed","generation"];
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
enum Kind {Candidate,Checkpoint,InProgress}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
 kind:Kind,campaign_name:String,campaign_id:String,campaign_status:String,
 #[serde(default)]phase:String,#[serde(default)]checkpoint:String,
 #[serde(default)]original_artifact_hash:String,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {format:String,version:u32,artifact_sha256:String,artifact:Value,source:Source,exported_ms:i64}
fn hex(s:&str,len:usize)->bool{s.len()==len&&s.bytes().all(|b|b.is_ascii_hexdigit())}
fn safe_name(s:&str)->bool{!s.is_empty()&&s.len()<=160&&s.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-')}
fn validate_artifact(value:&Value)->Result<Artifact>{
 let keys=value.as_object().ok_or_else(||anyhow::anyhow!("Netwerk ontbreekt"))?;
 ensure!(keys.keys().all(|k|FIELDS.contains(&k.as_str())),"Onbekende velden in netwerkbestand");
 let art:Artifact=serde_json::from_value(value.clone())?;art.validate()?;
 ensure!(hex(&art.dataset_hash,64),"Datasetversie ontbreekt of is ongeldig");
 ensure!(art.normalizer.start_ms>0&&art.normalizer.end_ms>art.normalizer.start_ms,"Normalisatieperiode ontbreekt");
 ensure!(art.instrument.len()<=64&&art.instrument.bytes().all(|b|b.is_ascii_uppercase()||b.is_ascii_digit()||b==b'-'),"Marktnaam ongeldig");
 ensure!(!art.laya||art.laya_runtime=="torch2.8-eager-mps-fp32-v1","Laya-rekenversie niet ondersteund");
 ensure!(art.laya||art.laya_checkpoint.is_empty()&&art.laya_runtime.is_empty(),"Laya-versie past niet bij dit netwerk");
 Ok(art)
}
fn digest(art:&Artifact)->Result<String>{Ok(neuro_data::hash(&serde_json::to_vec(art)?))}
fn validate(bundle:&Bundle)->Result<Artifact>{
 ensure!(bundle.format==FORMAT&&bundle.version==1,"Onbekend netwerkbestand of bestandsversie");
 ensure!(serde_json::to_vec(bundle)?.len()<=MAX_BYTES,"Netwerkbestand is te groot");
 let art=validate_artifact(&bundle.artifact)?;
 ensure!(hex(&bundle.artifact_sha256,64)&&digest(&art)?==bundle.artifact_sha256,"Netwerkhash wijkt af; bestand is gewijzigd of beschadigd");
 let s=&bundle.source;
 ensure!(!s.campaign_name.trim().is_empty()&&s.campaign_name.len()<=100&&hex(&s.campaign_id,32)&&s.campaign_status.len()<=48&&s.phase.len()<=160,"Herkomstgegevens ongeldig");
 ensure!(s.checkpoint.is_empty()||safe_name(&s.checkpoint),"Checkpointnaam ongeldig");
 ensure!(s.original_artifact_hash.is_empty()||hex(&s.original_artifact_hash,64),"Artifactreferentie ongeldig");
 Ok(art)
}
fn package(value:Value,source:Source)->Result<Bundle>{
 let art=validate_artifact(&value)?;
 let bundle=Bundle{format:FORMAT.into(),version:1,artifact_sha256:digest(&art)?,artifact:serde_json::to_value(&art)?,source,exported_ms:now_ms()};
 validate(&bundle)?;Ok(bundle)
}
fn source(config:&Value,id:String,status:String,kind:Kind)->Source {Source{kind,campaign_name:config["name"].as_str().unwrap_or("Netwerk").into(),campaign_id:id,campaign_status:status,phase:String::new(),checkpoint:String::new(),original_artifact_hash:String::new()}}
pub fn router()->Router<App>{Router::new()
 .route("/api/training/networks",get(list))
 .route("/api/training/networks/import",post(import).layer(DefaultBodyLimit::max(MAX_BYTES)))
 .route("/api/training/networks/{id}",get(saved))
 .route("/api/training/artifacts/{hash}/export",get(export_candidate))
 .route("/api/training/campaigns/{id}/export",get(export_progress))
}
async fn list(State(a):State<App>)->Response{reply(async{
 let rows=sqlx::query("SELECT id,name,artifact_sha256,bundle->'artifact' AS artifact,bundle->'source' AS source,created_ms FROM training.network_imports ORDER BY created_ms DESC LIMIT 100").fetch_all(&a.db).await?;
 Ok(json!({"networks":rows.iter().map(|r|{let art:Value=r.get("artifact");json!({"id":r.get::<String,_>("id"),"name":r.get::<String,_>("name"),"artifact_sha256":r.get::<String,_>("artifact_sha256"),"instrument":art["instrument"],"bar":art["bar"],"generation":art["generation"],"seed":art["seed"],"source":r.get::<Value,_>("source"),"created_ms":r.get::<i64,_>("created_ms")})}).collect::<Vec<_>>()}))
 }.await)}
async fn saved(State(a):State<App>,Path(id):Path<String>)->Response{reply(async{
 ensure!(hex(&id,32),"Ongeldig netwerk-id");
 let bundle:Value=sqlx::query_scalar("SELECT bundle FROM training.network_imports WHERE id=$1").bind(&id).fetch_one(&a.db).await?;
 Ok(bundle)
 }.await)}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]struct Import {bundle:Bundle,name:String}
async fn import(State(a):State<App>,Json(r):Json<Import>)->Response{reply(async{
 let art=validate(&r.bundle)?;ensure!(!r.name.trim().is_empty()&&r.name.len()<=100,"Geef het netwerk een korte naam");
 let mut bytes=[0u8;16];use ring::rand::SecureRandom;ring::rand::SystemRandom::new().fill(&mut bytes).expect("OS random");
 let new_id:String=bytes.iter().map(|b|format!("{b:02x}")).collect();
 let mut tx=a.db.begin().await?;
 let id=sqlx::query_scalar::<_,String>("INSERT INTO training.network_imports(id,artifact_sha256,name,bundle,created_ms) VALUES($1,$2,$3,$4,$5) ON CONFLICT(artifact_sha256) DO NOTHING RETURNING id").bind(&new_id).bind(&r.bundle.artifact_sha256).bind(r.name.trim()).bind(serde_json::to_value(&r.bundle)?).bind(now_ms()).fetch_optional(&mut *tx).await?;
 let duplicate=id.is_none();let id=match id {Some(id)=>id,None=>sqlx::query_scalar("SELECT id FROM training.network_imports WHERE artifact_sha256=$1").bind(&r.bundle.artifact_sha256).fetch_one(&mut *tx).await?};
 sqlx::query("INSERT INTO training.audit(kind,subject,detail,at_ms) VALUES('NETWORK_IMPORTED',$1,$2,$3)").bind(&id).bind(json!({"artifact_sha256":r.bundle.artifact_sha256,"duplicate":duplicate,"scope":"research copy; no qualification, account limits or trading permission imported"})).bind(now_ms()).execute(&mut *tx).await?;
 tx.commit().await?;
 Ok(json!({"id":id,"duplicate":duplicate,"weights":WEIGHTS,"instrument":art.instrument,"bar":art.bar,"generation":art.generation,"research_only":true}))
 }.await)}
async fn export_candidate(State(a):State<App>,Path(hash):Path<String>)->Response{reply(async{
 ensure!(hex(&hash,64),"Ongeldige artifactreferentie");
 let row=sqlx::query("SELECT a.artifact,a.campaign_id,c.config,c.status FROM training.artifacts a JOIN training.campaigns c ON c.id=a.campaign_id WHERE a.hash=$1").bind(&hash).fetch_one(&a.db).await?;
 let mut src=source(&row.get::<Value,_>("config"),row.get("campaign_id"),row.get("status"),Kind::Candidate);src.original_artifact_hash=hash;
 Ok(serde_json::to_value(package(row.get("artifact"),src)?)?)
 }.await)}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]struct ExportQuery {#[serde(default)]checkpoint:String}
async fn export_progress(State(a):State<App>,Path(id):Path<String>,Query(q):Query<ExportQuery>)->Response{reply(async{
 ensure!(hex(&id,32),"Ongeldig campagne-id");
 let row=sqlx::query("SELECT config,status FROM training.campaigns WHERE id=$1").bind(&id).fetch_one(&a.db).await?;
 let dir=training::root().join("campaigns").join(&id);
 let (value,kind)=if q.checkpoint.is_empty(){
  let p=read(dir.join("progress.json"),8_000_000).await?;
  ensure!(p["best_artifact"].is_object(),"Er zijn nog geen getrainde gewichten. Export is mogelijk na de eerste berekende generatie.");
  (p,Kind::InProgress)
 }else{
  ensure!(safe_name(&q.checkpoint),"Checkpointnaam ongeldig");
  (read(dir.join("generations").join(format!("{}.json",q.checkpoint)),64_000).await?,Kind::Checkpoint)
 };
 let mut src=source(&row.get::<Value,_>("config"),id,row.get("status"),kind);
 src.phase=value["phase"].as_str().or(value["metrics"]["phase"].as_str()).unwrap_or("").into();src.checkpoint=q.checkpoint;
 let art=value.get("artifact").or_else(||value.get("best_artifact")).cloned().ok_or_else(||anyhow::anyhow!("Dit checkpoint bevat geen netwerk"))?;
 Ok(serde_json::to_value(package(art,src)?)?)
 }.await)}
async fn read(path:std::path::PathBuf,max:u64)->Result<Value>{
 ensure!(tokio::fs::metadata(&path).await?.len()<=max,"Checkpointbestand te groot");
 Ok(serde_json::from_slice(&tokio::fs::read(path).await?)?)
}
#[cfg(test)]mod tests{
 use super::*;
 fn artifact()->Value{let a=Artifact{schema:1,feature_version:crate::neuro::FEATURES.into(),weights:(0..WEIGHTS).map(|i|(i as f64*0.773916936745491).sin()*7.9).collect(),parameters:crate::neuro::Parameters::default(),normalizer:crate::neuro::Normalizer{mean:[0.12345678912345678;16],scale:[0.03124395937294501;16],start_ms:1_700_000_000_000,end_ms:1_710_000_000_000},laya:false,laya_checkpoint:String::new(),laya_runtime:String::new(),instrument:"BTC-EUR".into(),bar:"5m".into(),dataset_hash:"ab".repeat(32),seed:17,generation:3};serde_json::to_value(a).unwrap()}
 fn bundle()->Bundle{package(artifact(),Source{kind:Kind::Checkpoint,campaign_name:"TEST · kunstmatig netwerk".into(),campaign_id:"cd".repeat(16),campaign_status:"TRAINING".into(),phase:"fold-0".into(),checkpoint:"fold-0-17-3".into(),original_artifact_hash:String::new()}).unwrap()}
 #[test]fn exact_weights_normalizer_and_decisions_survive_json_roundtrip(){let b=bundle();let original=validate(&b).unwrap();let encoded=serde_json::to_vec_pretty(&b).unwrap();let decoded:Bundle=serde_json::from_slice(&encoded).unwrap();let restored=validate(&decoded).unwrap();assert_eq!(original.weights.iter().map(|x|x.to_bits()).collect::<Vec<_>>(),restored.weights.iter().map(|x|x.to_bits()).collect::<Vec<_>>());assert_eq!(original.normalizer.scale.map(f64::to_bits),restored.normalizer.scale.map(f64::to_bits));assert_eq!(serde_json::to_value(crate::neuro::activate(&original,[0.31;16])).unwrap(),serde_json::to_value(crate::neuro::activate(&restored,[0.31;16])).unwrap());}
 #[test]fn altered_weights_fail_integrity_check(){let mut b=bundle();b.artifact["weights"][12]=json!(0.12);assert!(validate(&b).is_err());}
 #[test]fn invalid_network_dimensions_normalization_and_parameters_fail(){for (field,value)in[("weights",json!([1.,2.])),("normalizer",json!({"mean":vec![0.;16],"scale":vec![0.;16],"start_ms":1,"end_ms":2})),("bar",json!("1h"))]{let mut b=bundle();b.artifact[field]=value;assert!(validate(&b).is_err());}}
 #[test]fn executable_or_permission_fields_are_not_accepted(){let mut b=bundle();b.artifact["live_release"]=json!({"confirmed":true});assert!(validate(&b).is_err());let mut v=serde_json::to_value(bundle()).unwrap();v["qualification"]=json!({"passed":true});assert!(serde_json::from_value::<Bundle>(v).is_err());}
 #[test]fn unknown_format_and_laya_versions_fail(){let mut b=bundle();b.version=2;assert!(validate(&b).is_err());let mut b=bundle();b.artifact["laya"]=json!(true);b.artifact["laya_checkpoint"]=json!(training::LAYA_PIN);b.artifact["laya_runtime"]=json!("other");assert!(validate(&b).is_err());}
 #[test]fn checkpoint_traversal_and_invalid_provenance_fail(){let mut b=bundle();b.source.checkpoint="../request".into();assert!(validate(&b).is_err());assert!(!safe_name("%2e%2e"));let mut b=bundle();b.source.campaign_id="unknown".into();assert!(validate(&b).is_err());}
}
