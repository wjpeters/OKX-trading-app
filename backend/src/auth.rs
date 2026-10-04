//! Optional local password gate, with expiring HttpOnly sessions. No public binding.
use crate::{App, model::now_ms};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use ring::{
    digest, pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use serde::Deserialize;
use serde_json::json;
use std::num::NonZeroU32;
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    s.as_bytes()
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok())
        .collect()
}
pub fn configured() -> bool {
    std::path::Path::new("/run/secrets/auth_password").is_file()
}
pub async fn allowed(a: &App, h: &HeaderMap) -> bool {
    if !configured() {
        return true;
    }
    let token = h
        .get(header::COOKIE)
        .and_then(|s| s.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(str::trim)
                .find_map(|v| v.strip_prefix("okx_session="))
        });
    let Some(token) = token else {
        return false;
    };
    if token.len() != 64 {
        return false;
    }
    let hash = hex(digest::digest(&digest::SHA256, token.as_bytes()).as_ref());
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM studio.sessions WHERE token_hash=$1 AND expires_ms>$2)",
    )
    .bind(hash)
    .bind(now_ms())
    .fetch_one(&a.db)
    .await
    .unwrap_or(false)
}
pub fn router() -> Router<App> {
    Router::new()
        .route("/api/auth/status", get(status))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
}
async fn status(State(a): State<App>, h: HeaderMap) -> Json<serde_json::Value> {
    Json(
        json!({"configured":configured(),"authenticated":allowed(&a,&h).await,"scope":"LOCALHOST_ONLY","session_hours":8}),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    password: String,
}
async fn login(State(a): State<App>, Json(r): Json<Login>) -> Response {
    if r.password.len() > 256 {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let valid = tokio::task::spawn_blocking(move || {
        let text = std::fs::read_to_string("/run/secrets/auth_password").unwrap_or_default();
        let parts: Vec<_> = text.trim().split(':').collect();
        if parts.len() != 2 {
            return false;
        }
        let Some(salt) = decode(parts[0]) else {
            return false;
        };
        let Some(expected) = decode(parts[1]) else {
            return false;
        };
        pbkdf2::verify(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(600000).unwrap(),
            &salt,
            r.password.as_bytes(),
            &expected,
        )
        .is_ok()
    })
    .await
    .unwrap_or(false);
    if !valid {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"Aanmelden mislukt"})),
        )
            .into_response();
    }
    let mut bytes = [0u8; 32];
    if SystemRandom::new().fill(&mut bytes).is_err() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let token = hex(&bytes);
    let hash = hex(digest::digest(&digest::SHA256, token.as_bytes()).as_ref());
    if sqlx::query("INSERT INTO studio.sessions(token_hash,expires_ms) VALUES($1,$2)")
        .bind(hash)
        .bind(now_ms() + 8 * 3600000)
        .execute(&a.db)
        .await
        .is_err()
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    sqlx::query("DELETE FROM studio.sessions WHERE expires_ms<$1")
        .bind(now_ms())
        .execute(&a.db)
        .await
        .ok();
    let mut response = Json(json!({"ok":true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        format!("okx_session={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=28800")
            .parse()
            .unwrap(),
    );
    response
}
async fn logout(State(a): State<App>, h: HeaderMap) -> Response {
    if let Some(token) = h
        .get(header::COOKIE)
        .and_then(|s| s.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(str::trim)
                .find_map(|v| v.strip_prefix("okx_session="))
        })
    {
        let hash = hex(digest::digest(&digest::SHA256, token.as_bytes()).as_ref());
        sqlx::query("DELETE FROM studio.sessions WHERE token_hash=$1")
            .bind(hash)
            .execute(&a.db)
            .await
            .ok();
    }
    let mut r = Json(json!({"ok":true})).into_response();
    r.headers_mut().insert(
        header::SET_COOKIE,
        "okx_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"
            .parse()
            .unwrap(),
    );
    r
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_hash_fails() {
        assert!(decode("abc").is_none());
        assert!(decode("zz").is_none());
        assert_eq!(decode("00ff"), Some(vec![0, 255]));
    }
}
