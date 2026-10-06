use crate::{model::Candle, neuro};
use anyhow::{Result,ensure};
use parquet::{basic::Compression,data_type::{ByteArray,ByteArrayType,Int64Type},file::{properties::WriterProperties,writer::SerializedFileWriter,reader::{FileReader,SerializedFileReader}},record::RowAccessor,schema::parser::parse_message_type};
use std::{collections::BTreeMap,path::Path,sync::Arc};
pub fn hash(bytes:&[u8])->String {ring::digest::digest(&ring::digest::SHA256,bytes).as_ref().iter().map(|b|format!("{b:02x}")).collect()}
pub fn encode(cs:&[Candle])->Result<Vec<u8>> {
 let schema=Arc::new(parse_message_type("message candles { REQUIRED INT64 ts; REQUIRED BINARY open (UTF8); REQUIRED BINARY high (UTF8); REQUIRED BINARY low (UTF8); REQUIRED BINARY close (UTF8); REQUIRED BINARY volume (UTF8); }")?);
 let props=Arc::new(WriterProperties::builder().set_compression(Compression::SNAPPY).build());
 let mut writer=SerializedFileWriter::new(Vec::new(),schema,props)?;
 for batch in cs.chunks(10000){let mut group=writer.next_row_group()?;let mut time=group.next_column()?.unwrap();time.typed::<Int64Type>().write_batch(&batch.iter().map(|c|c.ts).collect::<Vec<_>>(),None,None)?;time.close()?;
  for k in 0..5{let values:Vec<ByteArray>=batch.iter().map(|c|match k{0=>c.open,1=>c.high,2=>c.low,3=>c.close,_=>c.volume}.to_string().as_str().into()).collect();let mut col=group.next_column()?.unwrap();col.typed::<ByteArrayType>().write_batch(&values,None,None)?;col.close()?;}group.close()?;
 }Ok(writer.into_inner()?)
}
pub fn decode(path:&Path)->Result<Vec<Candle>> {
 let reader=SerializedFileReader::new(std::fs::File::open(path)?)?;ensure!(reader.metadata().file_metadata().num_rows()<=2_000_000,"Dataset te groot voor geheugenprofiel");
 let mut out=Vec::new();for row in reader.get_row_iter(None)?{let r=row?;out.push(Candle{ts:r.get_long(0)?,open:r.get_string(1)?.parse()?,high:r.get_string(2)?.parse()?,low:r.get_string(3)?.parse()?,close:r.get_string(4)?.parse()?,volume:r.get_string(5)?.parse()?,closed:true});}Ok(out)
}
pub fn load(path:&Path,bar:&str)->Result<neuro::Series> {let cs=decode(path)?;let step=crate::model::interval(bar).ok_or_else(||anyhow::anyhow!("Timeframe ongeldig"))?;neuro::Series::new(neuro::aggregate(&cs,step)?,step)}
pub fn laya_map(value:&serde_json::Value)->Result<BTreeMap<i64,[f64;4]>> {let mut out=BTreeMap::new();for r in value.as_array().ok_or_else(||anyhow::anyhow!("Laya-featurecache ongeldig"))?{let ts=r["ts"].as_i64().ok_or_else(||anyhow::anyhow!("Laya-tijd ontbreekt"))?;let vals:[f64;4]=serde_json::from_value(r["values"].clone())?;ensure!(vals.iter().all(|v|v.is_finite()&&(0.0..=1.).contains(v)),"Laya-probabilities ongeldig");out.insert(ts,vals);}Ok(out)}
