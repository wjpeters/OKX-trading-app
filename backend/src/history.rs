//! Resumable, bounded historical pages archived as compressed Parquet.
use crate::{
    App, CandleQuery, candle_result,
    model::{Candle, interval, now_ms},
    studio::reply,
};
use anyhow::{Result, ensure};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use parquet::{
    basic::Compression,
    data_type::{ByteArray, ByteArrayType, Int64Type},
    file::{properties::WriterProperties, writer::SerializedFileWriter},
    schema::parser::parse_message_type,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;
pub fn router() -> Router<App> {
    Router::new()
        .route("/api/history", get(list).post(import))
        .route("/api/history/export", get(export))
}
fn encode(cs: &[Candle]) -> Result<Vec<u8>> {
    let schema = Arc::new(parse_message_type(
        "message candles { REQUIRED INT64 ts; REQUIRED BINARY open (UTF8); REQUIRED BINARY high (UTF8); REQUIRED BINARY low (UTF8); REQUIRED BINARY close (UTF8); REQUIRED BINARY volume (UTF8); }",
    )?);
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build(),
    );
    let mut writer = SerializedFileWriter::new(Vec::new(), schema, props)?;
    let mut group = writer.next_row_group()?;
    let mut time = group.next_column()?.unwrap();
    time.typed::<Int64Type>().write_batch(
        &cs.iter().map(|c| c.ts).collect::<Vec<_>>(),
        None,
        None,
    )?;
    time.close()?;
    for k in 0..5 {
        let values: Vec<ByteArray> = cs
            .iter()
            .map(|c| {
                match k {
                    0 => c.open,
                    1 => c.high,
                    2 => c.low,
                    3 => c.close,
                    _ => c.volume,
                }
                .to_string()
                .as_str()
                .into()
            })
            .collect();
        let mut col = group.next_column()?.unwrap();
        col.typed::<ByteArrayType>()
            .write_batch(&values, None, None)?;
        col.close()?;
    }
    group.close()?;
    Ok(writer.into_inner()?)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    instrument: String,
    bar: String,
    pages: u32,
    resume: Option<bool>,
}
async fn import(State(a): State<App>, Json(r): Json<Import>) -> Response {
    reply(async{ensure!((1..=10).contains(&r.pages),"Importeer 1–10 pagina's per opdracht");let _permit=a.replays.clone().try_acquire_owned().map_err(|_|anyhow::anyhow!("Historie/replay is al bezig"))?;let mut after:Option<i64>=sqlx::query_scalar("SELECT min(first_ms) FROM studio.history_parts WHERE instrument=$1 AND bar=$2").bind(&r.instrument).bind(&r.bar).fetch_one(&a.db).await?;if r.resume==Some(false){after=None;}let mut ids=vec![];for _ in 0..r.pages{let data=candle_result(&a,&CandleQuery{instrument:r.instrument.clone(),bar:r.bar.clone(),after}).await?;let cs:Vec<Candle>=serde_json::from_value(data["candles"].clone())?;let closed:Vec<_>=cs.into_iter().filter(|c|c.closed).collect();if closed.is_empty(){break;}let first=closed.first().unwrap().ts;let last=closed.last().unwrap().ts;ensure!(after.is_none_or(|t|first<t),"Historische cursor maakte geen voortgang");let bytes=encode(&closed)?;let name=format!("{}-{}-{}-{}.parquet",r.instrument,r.bar,first,last);let path=format!("/archives/{name}");tokio::fs::write(format!("{path}.tmp"),bytes).await?;tokio::fs::rename(format!("{path}.tmp"),&path).await?;let id:i64=sqlx::query_scalar("INSERT INTO studio.history_parts(instrument,bar,first_ms,last_ms,rows,gaps,path,created_ms) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(instrument,bar,first_ms,last_ms) DO UPDATE SET path=$7 RETURNING id").bind(&r.instrument).bind(&r.bar).bind(first).bind(last).bind(closed.len() as i32).bind(data["gaps"].as_i64().unwrap_or(0) as i32).bind(name).bind(now_ms()).fetch_one(&a.db).await?;ids.push(id);after=Some(first);}Ok(json!({"parts":ids,"next_after":after,"source":"OKX closed candles","format":"Parquet Snappy; decimal prices stored as exact strings","coverage":"Only returned pages; gaps not filled"}))}.await)
}
async fn list(State(a): State<App>) -> Response {
    reply(async{let rows=sqlx::query("SELECT id,instrument,bar,first_ms,last_ms,rows,gaps FROM studio.history_parts ORDER BY id DESC LIMIT 200").fetch_all(&a.db).await?;Ok(json!(rows.iter().map(|r|json!({"id":r.get::<i64,_>("id"),"instrument":r.get::<String,_>("instrument"),"bar":r.get::<String,_>("bar"),"first_ms":r.get::<i64,_>("first_ms"),"last_ms":r.get::<i64,_>("last_ms"),"rows":r.get::<i32,_>("rows"),"gaps":r.get::<i32,_>("gaps")})).collect::<Vec<_>>()))}.await)
}
#[derive(Deserialize)]
struct Export {
    id: i64,
}
async fn export(State(a): State<App>, Query(r): Query<Export>) -> Response {
    match async {
        let name: String = sqlx::query_scalar("SELECT path FROM studio.history_parts WHERE id=$1")
            .bind(r.id)
            .fetch_one(&a.db)
            .await?;
        ensure!(
            !name.contains('/') && !name.contains(".."),
            "Archiefpad ongeldig"
        );
        let bytes = tokio::fs::read(format!("/archives/{name}")).await?;
        Ok::<_, anyhow::Error>(bytes)
    }
    .await
    {
        Ok(b) => (
            [
                (header::CONTENT_TYPE, "application/vnd.apache.parquet"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=okx-candles.parquet",
                ),
            ],
            b,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
fn decode(path: &str) -> Result<Vec<Candle>> {
    use parquet::{
        file::reader::{FileReader, SerializedFileReader},
        record::RowAccessor,
    };
    ensure!(
        !path.contains('/') && !path.contains(".."),
        "Ongeldig archiefpad"
    );
    let reader = SerializedFileReader::new(std::fs::File::open(format!("/archives/{path}"))?)?;
    ensure!(
        reader.metadata().file_metadata().num_rows() <= 300,
        "Archiefsegment te groot"
    );
    let mut out = Vec::new();
    for row in reader.get_row_iter(None)? {
        let row = row?;
        let c = Candle {
            ts: row.get_long(0)?,
            open: row.get_string(1)?.parse()?,
            high: row.get_string(2)?.parse()?,
            low: row.get_string(3)?.parse()?,
            close: row.get_string(4)?.parse()?,
            volume: row.get_string(5)?.parse()?,
            closed: true,
        };
        ensure!(
            c.low > rust_decimal::Decimal::ZERO
                && c.low <= c.open
                && c.low <= c.close
                && c.high >= c.open
                && c.high >= c.close
                && c.volume >= rust_decimal::Decimal::ZERO,
            "Ongeldige historische OHLCV"
        );
        out.push(c);
    }
    Ok(out)
}
pub async fn dataset(a: &App, instrument: &str, bar: &str) -> Result<Vec<Candle>> {
    let step = interval(bar).ok_or_else(|| anyhow::anyhow!("Timeframe ongeldig"))?;
    let data = candle_result(
        a,
        &CandleQuery {
            instrument: instrument.into(),
            bar: bar.into(),
            after: None,
        },
    )
    .await?;
    ensure!(data["gaps"] == 0, "Datagat in actuele dataset");
    let current: Vec<Candle> = serde_json::from_value(data["candles"].clone())?;
    let paths:Vec<String>=sqlx::query_scalar("SELECT path FROM studio.history_parts WHERE instrument=$1 AND bar=$2 ORDER BY last_ms DESC LIMIT 34").bind(instrument).bind(bar).fetch_all(&a.db).await?;
    let cs=tokio::task::spawn_blocking(move || -> Result<Vec<Candle>> {
        let mut all=std::collections::BTreeMap::new();
        for path in paths {for c in decode(&path)? {all.insert(c.ts,c);}}
        for c in current.into_iter().filter(|c|c.closed&&c.ts+step<=now_ms()) {all.insert(c.ts,c);}
        let cs:Vec<_>=all.into_values().rev().take(10000).collect::<Vec<_>>().into_iter().rev().collect();
        ensure!(cs.windows(2).all(|w|w[1].ts-w[0].ts==step),"Datagat tussen archief en actuele candles. Ververs eerst de recente historie; er worden geen prijzen aangevuld.");
        Ok(cs)
    }).await??;
    Ok(cs)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parquet_is_readable_and_exact() {
        use crate::risk::d;
        use parquet::file::reader::{FileReader, SerializedFileReader};
        let c = Candle {
            ts: 123,
            open: d("1.000000000001"),
            high: d("2"),
            low: d("1"),
            close: d("1.5"),
            volume: d("3"),
            closed: true,
        };
        let bytes = encode(&[c]).unwrap();
        let path =
            std::env::temp_dir().join(format!("okx-parquet-test-{}.parquet", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        let reader = SerializedFileReader::new(std::fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(reader.metadata().file_metadata().num_rows(), 1);
        let row = reader.get_row_iter(None).unwrap().next().unwrap().unwrap();
        assert!(row.to_string().contains("1.000000000001"));
        std::fs::remove_file(path).unwrap();
    }
}
