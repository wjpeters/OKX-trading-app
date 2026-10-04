//! Private exchange transport. Compiled only into the separate onboarding/gateway binary.
//! No browser, Pine worker or paper terminal imports this module.
use anyhow::{Result, ensure};
use base64::{Engine, prelude::BASE64_STANDARD};
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Mode {
    OkxDemo,
    Live,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    api_key: String,
    secret: String,
    passphrase: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Readiness {
    pub mode: Mode,
    pub account_uid: String,
    pub eea_confirmed: bool,
    pub instruments_confirmed: bool,
    pub budget_confirmed: bool,
    pub limits_confirmed: bool,
    pub acceptance_complete: bool,
    pub exclusive_gateway: bool,
    pub reconciled: bool,
    pub native_protection_verified: bool,
}
impl Readiness {
    pub fn may_enter(&self) -> Result<()> {
        ensure!(
            !self.account_uid.is_empty()
                && self.eea_confirmed
                && self.instruments_confirmed
                && self.budget_confirmed
                && self.limits_confirmed
                && self.acceptance_complete
                && self.exclusive_gateway
                && self.reconciled
                && self.native_protection_verified,
            "Account, instrumenten, budget, limieten, exclusieve gateway, reconciliatie, bescherming en technische acceptatie moeten expliciet bevestigd zijn"
        );
        Ok(())
    }
}
pub fn sign(secret: &str, timestamp: &str, method: &str, path: &str, body: &str) -> String {
    BASE64_STANDARD.encode(
        hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes()),
            format!("{timestamp}{method}{path}{body}").as_bytes(),
        )
        .as_ref(),
    )
}
fn allowed(method: &str, path: &str) -> bool {
    let p = path.split('?').next().unwrap_or("");
    match method {
        "GET" => [
            "/api/v5/account/config",
            "/api/v5/account/instruments",
            "/api/v5/account/trade-fee",
            "/api/v5/account/balance",
            "/api/v5/account/positions",
            "/api/v5/trade/orders-pending",
            "/api/v5/trade/fills",
            "/api/v5/trade/order",
            "/api/v5/trade/orders-algo-pending",
            "/api/v5/public/time",
        ]
        .contains(&p),
        "POST" => [
            "/api/v5/trade/order",
            "/api/v5/trade/cancel-order",
            "/api/v5/trade/amend-order",
            "/api/v5/trade/order-algo",
            "/api/v5/trade/cancel-algos",
            "/api/v5/trade/cancel-all-after",
        ]
        .contains(&p),
        _ => false,
    }
}
pub struct Okx {
    mode: Mode,
    credentials: Credentials,
    http: reqwest::Client,
    requests: Mutex<Instant>,
}
impl Okx {
    pub fn new(mode: Mode, credentials: Credentials) -> Result<Self> {
        ensure!(
            !credentials.api_key.is_empty()
                && !credentials.secret.is_empty()
                && !credentials.passphrase.is_empty(),
            "Onvolledige credentials"
        );
        Ok(Self {
            mode,
            credentials,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            requests: Mutex::new(Instant::now()),
        })
    }
    async fn request(&self, method: &str, path: &str, payload: Option<&Value>) -> Result<Value> {
        ensure!(allowed(method, path), "Endpoint niet toegestaan");
        let mut throttle = self.requests.lock().await;
        tokio::time::sleep(Duration::from_millis(250).saturating_sub(throttle.elapsed())).await;
        *throttle = Instant::now();
        let ts = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let body = payload.map(Value::to_string).unwrap_or_default();
        let signature = sign(&self.credentials.secret, &ts, method, path, &body);
        let mut request = self
            .http
            .request(method.parse()?, format!("https://eea.okx.com{path}"))
            .header("OK-ACCESS-KEY", &self.credentials.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", ts)
            .header("OK-ACCESS-PASSPHRASE", &self.credentials.passphrase)
            .header("Content-Type", "application/json");
        if self.mode == Mode::OkxDemo {
            request = request.header("x-simulated-trading", "1");
        }
        if method == "POST" {
            request = request.body(body).header(
                "expTime",
                (chrono::Utc::now().timestamp_millis() + 3000).to_string(),
            );
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("EXCHANGE_TRANSPORT_UNKNOWN: reconcile before retry"))?
            .error_for_status()
            .map_err(|_| {
                anyhow::anyhow!("EXCHANGE_HTTP_ERROR: reconcile mutating request before retry")
            })?;
        let mut bytes = vec![];
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= 4_000_000,
                "Exchange response limit"
            );
            bytes.extend(chunk);
        }
        let v: Value = serde_json::from_slice(&bytes)?;
        ensure!(v["code"] == "0", "OKX response code {}", v["code"]);
        Ok(v)
    }
    pub async fn snapshot(&self) -> Result<Value> {
        let started = Instant::now();
        let time = self.request("GET", "/api/v5/public/time", None).await?;
        let now = chrono::Utc::now().timestamp_millis();
        let exchange = time["data"][0]["ts"]
            .as_str()
            .unwrap_or("0")
            .parse::<i64>()?;
        ensure!((now - exchange).abs() < 1000, "Klokafwijking te groot");
        let config = self.request("GET", "/api/v5/account/config", None).await?;
        let instruments = self
            .request("GET", "/api/v5/account/instruments?instType=SPOT", None)
            .await?;
        let balance = self.request("GET", "/api/v5/account/balance", None).await?;
        let positions = self
            .request("GET", "/api/v5/account/positions", None)
            .await?;
        let orders = self
            .request("GET", "/api/v5/trade/orders-pending?instType=SPOT", None)
            .await?;
        let fills = self
            .request("GET", "/api/v5/trade/fills?instType=SPOT&limit=100", None)
            .await?;
        let fee_instrument = instruments["data"]
            .as_array()
            .and_then(|rows| {
                rows.iter().find_map(|i| {
                    i["instId"].as_str().filter(|id| {
                        id.ends_with("-EUR")
                            && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    })
                })
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Geen accounttoegankelijk EUR-spotinstrument gevonden; geen regiofallback"
                )
            })?;
        let fees = self
            .request(
                "GET",
                &format!("/api/v5/account/trade-fee?instType=SPOT&instId={fee_instrument}"),
                None,
            )
            .await?;
        Ok(
            json!({"mode":self.mode,"configuration":config["data"],"instruments":instruments["data"],"balance":balance["data"],"positions":positions["data"],"open_orders":orders["data"],"recent_fills":fills["data"],"fees":fees["data"],"received_ms":now,"roundtrip_ms":started.elapsed().as_secs_f64()*1000.,"entries_allowed":false,"reason":"Onboarding snapshot is not execution approval"}),
        )
    }
    pub fn websocket_url(&self) -> &'static str {
        match self.mode {
            Mode::OkxDemo => "wss://wseeapap.okx.com/ws/v5/private",
            Mode::Live => "wss://wseea.okx.com/ws/v5/private",
        }
    }
    pub fn websocket_login(&self) -> Value {
        let ts = chrono::Utc::now().timestamp().to_string();
        json!({"op":"login","args":[{"apiKey":self.credentials.api_key,"passphrase":self.credentials.passphrase,"timestamp":ts,"sign":sign(&self.credentials.secret,&ts,"GET","/users/self/verify","")}]})
    }
    // The transport has no public general POST method. A sealed approval is required.
    /// A bounded read-only capability probe. It grants no execution permission.
    pub async fn private_probe(&self) -> Result<Value> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::{
            connect_async_with_config,
            tungstenite::{Message, protocol::WebSocketConfig},
        };
        let config = WebSocketConfig::default()
            .max_message_size(Some(512_000))
            .max_frame_size(Some(512_000));
        let (mut ws, _) = tokio::time::timeout(
            Duration::from_secs(5),
            connect_async_with_config(self.websocket_url(), Some(config), false),
        )
        .await??;
        ws.send(Message::Text(self.websocket_login().to_string().into()))
            .await?;
        let login = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Private verbinding gesloten"))??;
        let v: Value = serde_json::from_str(login.to_text()?)?;
        ensure!(
            v["event"] == "login" && v["code"] == "0",
            "Private login afgewezen"
        );
        ws.send(Message::Text(json!({"op":"subscribe","args":[{"channel":"orders","instType":"SPOT"},{"channel":"account"},{"channel":"positions","instType":"ANY"}]}).to_string().into())).await?;
        let mut channels = std::collections::BTreeSet::new();
        let mut updates = 0;
        let started = Instant::now();
        while channels.len() < 3 {
            ensure!(
                started.elapsed() < Duration::from_secs(15),
                "Private kanaalbevestiging niet ontvangen"
            );
            let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Private verbinding gesloten"))??;
            match msg {
                Message::Text(t) => {
                    if t == "pong" {
                        continue;
                    }
                    let v: Value = serde_json::from_str(&t)?;
                    ensure!(
                        v["event"] != "error",
                        "Private kanaal afgewezen; controleer accountrechten"
                    );
                    if v["event"] == "subscribe" {
                        if let Some(c) = v["arg"]["channel"].as_str() {
                            ensure!(
                                ["orders", "account", "positions"].contains(&c),
                                "Onverwacht kanaal"
                            );
                            channels.insert(c.to_owned());
                        }
                    } else if v["data"].is_array() {
                        updates += 1;
                        ensure!(updates < 1000, "Private buffer overbelast");
                    }
                }
                Message::Ping(p) => {
                    ws.send(Message::Pong(p)).await?;
                }
                Message::Close(_) => anyhow::bail!("Private verbinding gesloten"),
                _ => {}
            }
        }
        ws.close(None).await.ok();
        Ok(
            json!({"state":"READ_ONLY_SUBSCRIPTIONS_VERIFIED","channels":channels,"updates_observed":updates,"duration_ms":started.elapsed().as_secs_f64()*1000.,"execution_ready":false,"scope":"Capability probe only; no continuous reconciliation or order authority"}),
        )
    }
    pub async fn submit(&self, approval: &RiskApprovedOrder, ready: &Readiness) -> Result<Value> {
        ready.may_enter()?;
        ensure!(ready.mode == self.mode, "Omgevingen komen niet overeen");
        ensure!(
            approval.expires_ms >= chrono::Utc::now().timestamp_millis(),
            "Risk approval verlopen"
        );
        self.request("POST", "/api/v5/trade/order", Some(&approval.payload))
            .await
    }
    pub async fn reconcile_order(&self, instrument: &str, client_id: &str) -> Result<Value> {
        ensure!(
            instrument
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                && client_id.bytes().all(|c| c.is_ascii_alphanumeric()),
            "Ongeldige orderidentificatie"
        );
        self.request(
            "GET",
            &format!("/api/v5/trade/order?instId={instrument}&clOrdId={client_id}"),
            None,
        )
        .await
    }
}
// Constructor is inaccessible outside the fixed authorization module. Never Deserialize.
pub struct RiskApprovedOrder {
    payload: Value,
    expires_ms: i64,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Reconciler {
    pub last_update: i64,
    pub cumulative_fill: std::collections::BTreeMap<String, String>,
    pub unknown: std::collections::BTreeSet<String>,
    pub external: std::collections::BTreeSet<String>,
    pub synchronized: bool,
}
impl Reconciler {
    pub fn disconnected(&mut self) {
        self.synchronized = false;
    }
    pub fn uncertain(&mut self, id: &str) {
        self.unknown.insert(id.into());
        self.synchronized = false;
    }
    pub fn update(
        &mut self,
        order: &Value,
        known: &std::collections::BTreeSet<String>,
    ) -> Result<bool> {
        use rust_decimal::Decimal as D;
        use std::str::FromStr;
        let id = order["clOrdId"].as_str().unwrap_or("");
        if !known.contains(id) {
            self.external
                .insert(order["ordId"].as_str().unwrap_or("unidentified").into());
            self.synchronized = false;
            return Ok(false);
        }
        let qty = D::from_str(order["accFillSz"].as_str().unwrap_or(""))?;
        ensure!(qty >= D::ZERO, "Negatieve fill");
        let old = self
            .cumulative_fill
            .get(id)
            .and_then(|q| D::from_str(q).ok())
            .unwrap_or(D::ZERO);
        if qty < old {
            return Ok(false);
        }
        let changed = qty > old;
        self.cumulative_fill.insert(id.into(), qty.to_string());
        if ["live", "partially_filled", "filled", "canceled"]
            .iter()
            .any(|s| order["state"] == *s)
        {
            self.unknown.remove(id);
        }
        self.last_update = chrono::Utc::now().timestamp_millis();
        Ok(changed)
    }
    pub fn may_retry(&self, id: &str) -> bool {
        !self.unknown.contains(id)
            && !self.cumulative_fill.contains_key(id)
            && self.synchronized
            && self.external.is_empty()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn withdraw_never_allowed() {
        assert!(!allowed("POST", "/api/v5/asset/withdrawal"));
        assert!(!allowed("POST", "https://evil.invalid/api/v5/trade/order"));
    }
    #[test]
    fn live_readiness_missing_acceptance_fails() {
        let r = Readiness {
            mode: Mode::Live,
            account_uid: "x".into(),
            eea_confirmed: true,
            instruments_confirmed: true,
            budget_confirmed: true,
            limits_confirmed: true,
            acceptance_complete: false,
            exclusive_gateway: true,
            reconciled: true,
            native_protection_verified: true,
        };
        assert!(r.may_enter().is_err());
    }
    #[test]
    fn signing_fixture() {
        assert_eq!(
            sign(
                "secret",
                "2020-12-08T09:08:57.715Z",
                "GET",
                "/api/v5/account/balance?ccy=BTC",
                ""
            ),
            "wpDvCwYCprcMQsQkxWJiWy+YADoQE4ep+OEKKLimMoY="
        );
    }
    #[test]
    fn accepted_timeout_never_resubmits() {
        let mut r = Reconciler::default();
        r.uncertain("a");
        assert!(!r.may_retry("a"));
        let known = ["a".to_string()].into_iter().collect();
        let v = json!({"clOrdId":"a","ordId":"1","accFillSz":"0.01","state":"partially_filled"});
        assert!(r.update(&v, &known).unwrap());
        assert!(!r.update(&v, &known).unwrap());
        assert!(!r.may_retry("a"));
    }
    #[test]
    fn external_order_blocks() {
        let mut r = Reconciler::default();
        r.synchronized = true;
        r.update(
            &json!({"ordId":"manual","clOrdId":"external"}),
            &Default::default(),
        )
        .unwrap();
        assert!(!r.synchronized);
    }
    #[test]
    fn known_filled_order_never_resubmitted() {
        let mut r = Reconciler::default();
        r.synchronized = true;
        let known = ["done".to_string()].into_iter().collect();
        r.update(
            &json!({"clOrdId":"done","accFillSz":"1","state":"filled"}),
            &known,
        )
        .unwrap();
        assert!(!r.may_retry("done"));
    }
    #[test]
    fn old_fill_ignored() {
        let mut r = Reconciler::default();
        let known = ["a".to_string()].into_iter().collect();
        r.update(
            &json!({"clOrdId":"a","accFillSz":"2","state":"filled"}),
            &known,
        )
        .unwrap();
        assert!(
            !r.update(
                &json!({"clOrdId":"a","accFillSz":"1","state":"partially_filled"}),
                &known
            )
            .unwrap()
        );
        assert_eq!(r.cumulative_fill["a"], "2");
    }
}
