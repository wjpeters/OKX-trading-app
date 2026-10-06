mod auth;
mod book;
mod comparison;
mod feeds;
mod graph;
mod history;
mod neuro;
mod native_bot_risk;
mod native_bots;
mod neuro_data;
mod training;
mod training_transfer;
mod training_feed;
mod indicators;
mod model;
mod operations;
mod order_types;
mod order_risk;
mod order_wallet;
mod trading;
mod paper;
#[allow(dead_code)]
mod proposal;
mod providers;
#[allow(dead_code)]
mod risk;
mod secrets;
mod settings;
mod studio;
use anyhow::{Result, bail};
use axum::{
    Json, Router,
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use model::{Timeframe, normalize, now_ms};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{
    PgPool, Row,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
pub struct App {
    pub market: Arc<RwLock<Market>>,
    pub http: reqwest::Client,
    pub throttle: Arc<Mutex<Instant>>,
    pub db: PgPool,
    pub candle_lock: Arc<Mutex<()>>,
    pub sockets: Arc<Semaphore>,
    pub metrics: Arc<Mutex<VecDeque<f64>>>,
    pub paper_heartbeat: Arc<std::sync::atomic::AtomicI64>,
    pub replays: Arc<Semaphore>,
    pub commands: Arc<Semaphore>,
    pub scripts: Arc<Semaphore>,
    pub request_times: Arc<Mutex<VecDeque<i64>>>,
}
#[derive(Default, serde::Serialize)]
pub struct Market {
    pub instruments: Vec<Value>,
    #[serde(skip_serializing)]
    pub catalogue: Vec<Value>,
    pub metadata_received_ms: Option<i64>,
    pub tickers: BTreeMap<String, Value>,
    pub books: BTreeMap<String, Value>,
    pub trades: BTreeMap<String, VecDeque<Value>>,
    pub feed_connected: bool,
    pub feed_error: Option<String>,
    pub reconnects: u64,
    pub validated_messages: u64,
    pub last_message_ms: Option<i64>,
    pub clock_offset_ms: Option<i64>,
    pub clock_rtt_ms: Option<i64>,
}
impl App {
    pub async fn public_get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value> {
        if ![
            "/api/v5/public/time",
            "/api/v5/public/instruments",
            "/api/v5/market/candles",
            "/api/v5/market/history-candles",
        ]
        .contains(&path)
        {
            bail!("Endpoint niet toegestaan");
        }
        let mut throttle = self.throttle.lock().await;
        let delay = Duration::from_millis(150).saturating_sub(throttle.elapsed());
        tokio::time::sleep(delay).await;
        *throttle = Instant::now();
        let response = self
            .http
            .get(format!("https://eea.okx.com{path}"))
            .query(query)
            .send()
            .await?
            .error_for_status()?;
        if response.content_length().unwrap_or(0) > 8_000_000 {
            bail!("Response te groot");
        }
        let body: Value = response.json().await?;
        if body["code"] != "0" {
            bail!("OKX foutcode {}", body["code"]);
        }
        if !body["data"].is_array() {
            bail!("Onverwacht OKX schema");
        }
        Ok(body)
    }
}
async fn market_catalogue(State(app):State<App>)->Json<Value>{
    let m=app.market.read().await;Json(json!({"instruments":m.catalogue,"received_ms":m.metadata_received_ms}))
}
#[derive(Deserialize)]
struct WatchMarket { instrument:String }
async fn watch_market(State(app):State<App>,Json(request):Json<WatchMarket>)->Response {
    studio::reply(async {
        let mut m=app.market.write().await;
        let meta=m.catalogue.iter().find(|i|i["instId"]==request.instrument).cloned().ok_or_else(||anyhow::anyhow!("Spotmarkt niet beschikbaar"))?;
        if !m.instruments.iter().any(|i|i["instId"]==request.instrument) {
            if m.instruments.len()>=44 {
                if let Some(index)=m.instruments.iter().position(|i|!feeds::PREFERRED.contains(&i["instId"].as_str().unwrap_or(""))) {
                    let old=m.instruments.remove(index);if let Some(id)=old["instId"].as_str(){m.books.remove(id);m.tickers.remove(id);m.trades.remove(id);}
                }
            }
            m.instruments.push(meta);
        }
        Ok(json!({"ok":true,"instrument":request.instrument}))
    }.await)
}
#[derive(Deserialize)]
struct CandleQuery {
    instrument: String,
    bar: String,
    after: Option<i64>,
}
async fn candles(State(app): State<App>, Query(q): Query<CandleQuery>) -> Response {
    match candle_result(&app, &q).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":e.to_string(),"stale":true})),
        )
            .into_response(),
    }
}
async fn candle_result(app: &App, q: &CandleQuery) -> Result<Value> {
    let timeframe =
        Timeframe::parse(&q.bar).ok_or_else(|| anyhow::anyhow!("Timeframe niet ondersteund"))?;
    if !app
        .market
        .read()
        .await
        .catalogue
        .iter()
        .any(|v| v["instId"] == q.instrument)
    {
        bail!("Spotmarkt niet in gecontroleerde catalogus");
    }
    if q.after.is_some_and(|v| v <= 0 || v > now_ms()) {
        bail!("Ongeldige historische cursor");
    }
    let _guard = app.candle_lock.lock().await;
    let cache_key = match q.after {
        Some(ts) => format!("{}:{ts}", q.bar),
        None => q.bar.clone(),
    };
    if let Some(row) = sqlx::query(
        "SELECT received_at_ms,payload FROM market_cache WHERE instrument=$1 AND timeframe=$2",
    )
    .bind(&q.instrument)
    .bind(&cache_key)
    .fetch_optional(&app.db)
    .await?
    {
        let age = now_ms() - row.get::<i64, _>("received_at_ms");
        if (0..10_000).contains(&age) {
            return Ok(row.get("payload"));
        }
    }
    let start = Instant::now();
    let after = q.after.map(|x| x.to_string());
    let mut query = vec![
        ("instId", q.instrument.as_str()),
        ("bar", q.bar.as_str()),
        ("limit", "300"),
    ];
    if let Some(v) = after.as_ref() {
        query.push(("after", v));
    }
    let endpoint = if q.after.is_some() {
        "/api/v5/market/history-candles"
    } else {
        "/api/v5/market/candles"
    };
    let raw = app.public_get(endpoint, &query).await?;
    let received_at_ms = now_ms();
    let (cs, gaps) = normalize(raw["data"].as_array().unwrap(), timeframe)?;
    if cs.is_empty() {
        bail!("Geen candles beschikbaar");
    }
    let calc = Instant::now();
    let points = indicators::calculate_for(&cs, timeframe, &indicators::Settings::default());
    let calculation_ms = calc.elapsed().as_secs_f64() * 1000.0;
    let fresh = q.after.is_none()
        && cs.last().is_some_and(|c| {
            c.ts <= received_at_ms + 3000
                && timeframe
                    .next_open(c.ts)
                    .is_some_and(|end| end + 15000 > received_at_ms)
        });
    let payload = json!({"instrument":q.instrument,"bar":q.bar,"source":"OKX EEA","received_at_ms":received_at_ms,"candles":cs,"indicators":points,"gaps":gaps,"fresh":fresh,"historical":q.after.is_some(),"rest_roundtrip_ms":start.elapsed().as_secs_f64()*1000.0,"calculation_ms":calculation_ms,"indicator_parameters":{"sma":20,"ema":20,"rsi":14,"macd":[12,26,9],"atr":14,"bb":[20,2],"vwap":"UTC day, partial if history starts intraday","stochastic":14,"adx":14,"obv":"relative to loaded history"}});
    sqlx::query("INSERT INTO market_cache(instrument,timeframe,received_at_ms,payload) VALUES($1,$2,$3,$4) ON CONFLICT(instrument,timeframe) DO UPDATE SET received_at_ms=$3,payload=$4").bind(&q.instrument).bind(&cache_key).bind(received_at_ms).bind(&payload).execute(&app.db).await?;
    Ok(payload)
}
async fn snapshot(State(app): State<App>) -> Json<Value> {
    Json(json!({"market":*app.market.read().await,"server_time_ms":now_ms()}))
}
async fn status(State(app): State<App>) -> Json<Value> {
    let db_ok = sqlx::query("SELECT 1").execute(&app.db).await.is_ok();
    let account = sqlx::query_scalar::<_, Value>("SELECT payload FROM paper_v2.account WHERE id=1")
        .fetch_optional(&app.db)
        .await
        .ok()
        .flatten();
    let worker_alive = now_ms()
        - app
            .paper_heartbeat
            .load(std::sync::atomic::Ordering::Relaxed)
        < 3000;
    let market_ready = {
        let m = app.market.read().await;
        m.feed_connected
            && m.clock_offset_ms.is_some_and(|v| v.abs() <= 1000)
            && m.clock_rtt_ms.is_some_and(|v| v <= 2000)
    };
    let risk_state = if !worker_alive {
        "WORKER_UNAVAILABLE"
    } else if account.is_none() {
        "SETUP_REQUIRED"
    } else if !market_ready
        || account
            .as_ref()
            .is_some_and(|p| p["valuation_fresh"] == false)
    {
        "PAUSE_ENTRIES_DATA"
    } else if account
        .as_ref()
        .is_some_and(|p| p["ledger"]["paused"] == true)
    {
        "PAUSE_ENTRIES"
    } else {
        "PAPER_READY"
    };
    let active_bots = account
        .as_ref()
        .and_then(|p| p["bots"].as_object())
        .map(|b| b.values().filter(|b| b["running"] == true).count())
        .unwrap_or(0);
    let jev_enabled: bool = sqlx::query_scalar("SELECT enabled FROM studio.ai_budget WHERE id=1")
        .fetch_optional(&app.db)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);
    let provider = settings::selected(&app).await.ok();
    let configured = if let Some(p) = provider {
        secrets::configured(p.key()).await
    } else {
        false
    };
    let jev_state = if !configured {
        "KEY_REQUIRED"
    } else if jev_enabled {
        "PROPOSALS_ENABLED"
    } else {
        "BUDGET_CONFIRMATION_REQUIRED"
    };
    let m = app.market.read().await;
    let mut samples: Vec<_> = app.metrics.lock().await.iter().copied().collect();
    samples.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        if samples.is_empty() {
            None
        } else {
            Some(samples[((samples.len() - 1) as f64 * p).round() as usize])
        }
    };
    Json(
        json!({"version":"0.4.0","phase":4,"mode":"PAPER_LOCAL","execution_enabled":worker_alive&&account.is_some(),"risk_state":risk_state,"risk_reason":account.as_ref().map(|p|p["ledger"]["reason"].clone()),"account":"Virtuele EUR-rekening","account_region":"UNVERIFIED","public_region":"EEA","database":db_ok,"public_ws":m.feed_connected,"last_message_ms":m.last_message_ms,"feed_error":m.feed_error,"reconnects":m.reconnects,"validated_messages":m.validated_messages,"instruments":m.instruments.len(),"clock_offset_ms":m.clock_offset_ms,"clock_rtt_ms":m.clock_rtt_ms,"active_bots":active_bots,"jev":{"state":jev_state,"model":provider.map(|p|p.model()),"provider":provider.map(|p|p.id()),"authority":"PROPOSALS_ONLY"},"processing_ms":{"p50":percentile(0.5),"p95":percentile(0.95),"p99":percentile(0.99),"samples":samples.len(),"scope":"WS validation + in-memory update; no order route"},"server_time_ms":now_ms()}),
    )
}
async fn health() -> Json<Value> {
    Json(json!({"alive":true}))
}
async fn ready(State(app): State<App>) -> StatusCode {
    if sqlx::query("SELECT 1").execute(&app.db).await.is_ok()
        && !app.market.read().await.instruments.is_empty()
        && app.market.read().await.feed_connected
        && now_ms()
            - app
                .paper_heartbeat
                .load(std::sync::atomic::Ordering::Relaxed)
            < 3000
    {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
fn allowed_host(host: &str) -> bool {
    [
        "127.0.0.1:4780",
        "localhost:4780",
        "127.0.0.1:4781",
        "localhost:4781",
        "127.0.0.1:8080",
        "localhost:8080",
        "127.0.0.1:5173",
        "localhost:5173",
    ]
    .contains(&host)
}
async fn boundary(State(app): State<App>, req: Request<axum::body::Body>, next: Next) -> Response {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if !allowed_host(host) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Some(origin) = req.headers().get("origin") {
        let o = origin.to_str().unwrap_or("");
        if !o.strip_prefix("http://").is_some_and(allowed_host) {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    let path = req.uri().path();
    if (path.starts_with("/api/") || path == "/ws")
        && ![
            "/api/health",
            "/api/ready",
            "/api/auth/status",
            "/api/auth/login",
        ]
        .contains(&path)
        && !auth::allowed(&app, req.headers()).await
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"Sessie verlopen; meld opnieuw aan"})),
        )
            .into_response();
    }
    let mutation =
        req.method() != axum::http::Method::GET && req.method() != axum::http::Method::HEAD;
    let _permit = if mutation {
        let origin = req.headers().get("origin").and_then(|h| h.to_str().ok());
        if origin != Some(format!("http://{host}").as_str())
            || req
                .headers()
                .get("x-terminal-request")
                .and_then(|h| h.to_str().ok())
                != Some("paper-v2")
        {
            return StatusCode::FORBIDDEN.into_response();
        }
        let Ok(permit) = app.commands.clone().try_acquire_owned() else {
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        };
        let mut times = app.request_times.lock().await;
        let now = now_ms();
        while times.front().is_some_and(|t| *t < now - 60000) {
            times.pop_front();
        }
        if times.len() >= 60 {
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        }
        times.push_back(now);
        Some(permit)
    } else {
        None
    };
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("referrer-policy", "no-referrer".parse().unwrap());
    headers.insert("cache-control", "no-store".parse().unwrap());
    headers.insert("content-security-policy","default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ws://127.0.0.1:4780 ws://localhost:4780 ws://127.0.0.1:4781 ws://localhost:4781; frame-ancestors 'none'; base-uri 'none'; form-action 'none'".parse().unwrap());
    res
}
async fn websocket(State(app): State<App>, headers: HeaderMap, ws: WebSocketUpgrade) -> Response {
    if headers.get("origin").is_none() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(permit) = app.sockets.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    ws.max_message_size(1024)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            stream(socket, app, headers).await
        })
        .into_response()
}
async fn stream(mut socket: WebSocket, app: App, headers: HeaderMap) {
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    let mut checked = Instant::now();
    loop {
        if checked.elapsed() > Duration::from_secs(30) {
            if !auth::allowed(&app, &headers).await {
                break;
            }
            checked = Instant::now();
        }
        tokio::select! {_ = interval.tick()=>{let body=json!({"market":*app.market.read().await,"server_time_ms":now_ms()}).to_string();if !matches!(tokio::time::timeout(Duration::from_secs(3),socket.send(Message::Text(body.into()))).await,Ok(Ok(()))){break;}},incoming=socket.recv()=>{match incoming{Some(Ok(Message::Close(_)))|None|Some(Err(_))=>break,_=>{}}}}
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("TLS-provider installatie mislukt"))?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();
    if std::env::var("EXECUTION_MODE").unwrap_or_else(|_| "PAPER_LOCAL".into()) != "PAPER_LOCAL" {
        bail!("Deze build ondersteunt uitsluitend PAPER_LOCAL; LIVE/DEMO-adapter ontbreekt");
    }
    let password = std::fs::read_to_string(
        std::env::var("DATABASE_PASSWORD_FILE")
            .unwrap_or_else(|_| "/run/secrets/db_password".into()),
    )?;
    let options = PgConnectOptions::new()
        .host(&std::env::var("DATABASE_HOST").unwrap_or_else(|_| "db".into()))
        .username("terminal")
        .database(&std::env::var("DATABASE_NAME").unwrap_or_else(|_| "terminal".into()))
        .password(password.trim())
        .options([
            ("statement_timeout", "3000ms"),
            ("lock_timeout", "1500ms"),
            ("idle_in_transaction_session_timeout", "5000ms"),
        ]);
    let db = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    sqlx::query("INSERT INTO audit_events(kind,detail) VALUES('START_PAPER_LOCAL', $1)")
        .bind(json!({"version":"0.4.0","mode":"PAPER_LOCAL","live_execution":false}))
        .execute(&db)
        .await?;
    let app = App {
        market: Arc::new(RwLock::new(Market::default())),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        throttle: Arc::new(Mutex::new(Instant::now())),
        db,
        candle_lock: Arc::new(Mutex::new(())),
        sockets: Arc::new(Semaphore::new(8)),
        metrics: Arc::new(Mutex::new(VecDeque::with_capacity(2048))),
        paper_heartbeat: Arc::new(std::sync::atomic::AtomicI64::new(0)),
        replays: Arc::new(Semaphore::new(1)),
        commands: Arc::new(Semaphore::new(4)),
        scripts: Arc::new(Semaphore::new(1)),
        request_times: Arc::new(Mutex::new(VecDeque::new())),
    };
    let worker_app = app.clone();
    tokio::spawn(async move {
        loop {
            let worker = tokio::spawn(feeds::run(worker_app.clone()));
            if let Err(error) = worker.await {
                tracing::error!(error=%error,"Market worker failed");
                let mut m = worker_app.market.write().await;
                m.feed_connected = false;
                m.feed_error = Some("Marktworker herstart na interne fout".into());
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
    tokio::spawn(feeds::refresh_clock(app.clone()));
    tokio::spawn(paper::run(app.clone()));
    tokio::spawn(paper::bots(app.clone()));
    tokio::spawn(studio::run(app.clone()));
    tokio::spawn(providers::run(app.clone()));
    tokio::spawn(operations::watch(app.clone()));
    tokio::spawn(training::run(app.clone()));
    tokio::spawn(native_bots::run(app.clone()));
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "/app/public".into());
    let router = Router::new()
        .merge(auth::router())
        .merge(paper::router())
        .merge(studio::router())
        .merge(graph::router())
        .merge(history::router())
        .merge(training::router())
        .merge(native_bots::router())
        .merge(operations::router())
        .merge(settings::router())
        .merge(trading::router())
        .route("/api/markets", get(market_catalogue))
        .route("/api/markets/watch", axum::routing::post(watch_market))
        .merge(providers::router())
        .merge(comparison::router())
        .route("/api/status", get(status))
        .route("/api/market", get(snapshot))
        .route("/api/candles", get(candles))
        .route("/api/health", get(health))
        .route("/api/ready", get(ready))
        .route("/ws", get(websocket))
        .route(
            "/api/{*path}",
            axum::routing::any(|| async { StatusCode::NOT_FOUND }),
        )
        .fallback_service(
            ServeDir::new(&static_dir)
                .not_found_service(ServeFile::new(format!("{static_dir}/index.html"))),
        )
        .layer(axum::extract::DefaultBodyLimit::max(16384))
        .layer(middleware::from_fn_with_state(app.clone(), boundary))
        .with_state(app);
    tracing::info!("Paper terminal listening; only local simulation adapter compiled");
    axum::serve(tokio::net::TcpListener::bind("0.0.0.0:8080").await?, router)
        .with_graceful_shutdown(async {
            #[cfg(unix)]
            {
                let mut term =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("SIGTERM handler");
                tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
        })
        .await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dns_rebinding_denied() {
        assert!(!allowed_host("evil.example:4780"));
        assert!(allowed_host("localhost:4780"));
    }
}
