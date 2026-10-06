//! Local credential management; never returns credential contents or contacts providers.
use crate::{App, secrets, studio::reply};
use anyhow::{Result, ensure};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    response::Response,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenRouter,
    TypeSafe,
}
impl Provider {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "openrouter" => Ok(Self::OpenRouter),
            "typesafe" => Ok(Self::TypeSafe),
            _ => anyhow::bail!("Kies Jev direct of Jev via OpenRouter"),
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::TypeSafe => "typesafe",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter_key",
            Self::TypeSafe => "typesafe_key",
        }
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::OpenRouter => "typesafe/jev-1.13",
            Self::TypeSafe => "jev-1.13.0",
        }
    }
    pub fn endpoint(self) -> &'static str {
        match self {
            Self::OpenRouter => "https://openrouter.ai/api/alpha/decisions",
            Self::TypeSafe => "https://api.typesafe.ai/v1/systemone",
        }
    }
}
pub async fn selected(a: &App) -> Result<Provider> {
    let p: String = sqlx::query_scalar("SELECT provider FROM studio.ai_budget WHERE id=1")
        .fetch_one(&a.db)
        .await?;
    Provider::parse(&p)
}
pub async fn lock(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<()> {
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(478006)")
        .fetch_one(&mut **tx)
        .await?;
    ensure!(locked, "Jev verwerkt nog een aanvraag. Probeer zo opnieuw.");
    Ok(())
}
pub fn router() -> Router<App> {
    Router::new()
        .route("/api/settings", get(status))
        .route("/api/settings/credentials", post(credentials))
        .route("/api/settings/provider", post(provider))
        .layer(DefaultBodyLimit::max(16384))
}
async fn status(State(a): State<App>) -> Response {
    reply(async {
        let p = selected(&a).await?;
        let b = sqlx::query("SELECT enabled,budget::text,spent::text,reserved::text FROM studio.ai_budget WHERE id=1").fetch_one(&a.db).await?;
        let mut keys = serde_json::Map::new();
        for name in ["okx_demo", "okx_live", "typesafe_key", "openrouter_key"] {
            keys.insert(name.into(), json!({"configured":secrets::configured(name).await}));
        }
        Ok(json!({"provider":p.id(),"model":p.model(),"credentials":keys,
            "storage_ready":std::env::var("SETTINGS_DIR").is_ok() && std::path::Path::new("/run/secrets/local_secrets_key").is_file(),
            "enabled":b.get::<bool,_>("enabled"),"budget_usd":b.get::<String,_>("budget"),
            "spent_usd":b.get::<String,_>("spent"),"reserved_usd":b.get::<String,_>("reserved")}))
    }.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    target: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    secret: String,
    #[serde(default)]
    passphrase: String,
    #[serde(default)]
    remove: bool,
}
fn valid_secret(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 2048 && !s.chars().any(char::is_control)
}
fn payload(r: &Credentials) -> Result<String> {
    ensure!(
        ["okx_demo", "okx_live", "typesafe_key", "openrouter_key"].contains(&r.target.as_str()),
        "Onbekende aansluiting"
    );
    if r.remove {
        ensure!(
            r.api_key.is_empty() && r.secret.is_empty() && r.passphrase.is_empty(),
            "Verwijderen mag geen sleutelvelden bevatten"
        );
        return Ok(String::new());
    }
    ensure!(
        valid_secret(&r.api_key),
        "Vul een geldige API-key in (maximaal 2048 tekens)"
    );
    if r.target.starts_with("okx_") {
        ensure!(
            valid_secret(&r.secret) && valid_secret(&r.passphrase),
            "Vul ook de OKX secret en passphrase in"
        );
        Ok(json!({"api_key":r.api_key.trim(),"secret":r.secret.trim(),"passphrase":r.passphrase.trim()}).to_string())
    } else {
        ensure!(
            r.secret.is_empty() && r.passphrase.is_empty(),
            "Deze provider gebruikt alleen een API-key"
        );
        Ok(r.api_key.trim().into())
    }
}
async fn credentials(
    State(a): State<App>,
    request: Result<Json<Credentials>, JsonRejection>,
) -> Response {
    reply(async {
        let Json(r) = request.map_err(|_| anyhow::anyhow!("Ongeldige sleutelvelden"))?;
        let value = payload(&r)?;
        let mut tx = a.db.begin().await?;
        lock(&mut tx).await?;
        // Credential changes require explicit AI reactivation after a successful save.
        if r.target.ends_with("_key") {
            sqlx::query("UPDATE studio.ai_budget SET enabled=false WHERE id=1").execute(&mut *tx).await?;
        }
        secrets::write(&r.target, &value).await.map_err(|_| anyhow::anyhow!("Lokaal opslaan mislukt. Controleer de opslag en start de terminal opnieuw via het startbestand."))?;
        tx.commit().await?;
        Ok(json!({"ok":true,"removed":r.remove}))
    }.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    provider: String,
}
async fn provider(
    State(a): State<App>,
    request: Result<Json<Selection>, JsonRejection>,
) -> Response {
    reply(async {
        let Json(r) = request.map_err(|_| anyhow::anyhow!("Ongeldige providerkeuze"))?;
        let p = Provider::parse(&r.provider)?;
        let mut tx = a.db.begin().await?;
        lock(&mut tx).await?;
        sqlx::query("UPDATE studio.ai_budget SET provider=$1,enabled=CASE WHEN provider=$1 THEN enabled ELSE false END WHERE id=1")
            .bind(p.id()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(json!({"ok":true,"provider":p.id()}))
    }.await)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_or_injected_secrets_are_rejected() {
        let mut r = Credentials {
            target: "okx_demo".into(),
            api_key: "fixture".into(),
            secret: String::new(),
            passphrase: String::new(),
            remove: false,
        };
        assert!(payload(&r).is_err());
        r.secret = "fixture-secret".into();
        r.passphrase = "fixture-pass".into();
        assert!(payload(&r).is_ok());
        r.api_key = "key\nheader".into();
        assert!(payload(&r).is_err());
        r.target = "../escape".into();
        assert!(payload(&r).is_err());
    }
    #[test]
    fn provider_destinations_are_fixed_and_distinct() {
        assert!(Provider::parse("https://other.example").is_err());
        assert_ne!(Provider::TypeSafe.key(), Provider::OpenRouter.key());
        assert_ne!(
            Provider::TypeSafe.endpoint(),
            Provider::OpenRouter.endpoint()
        );
    }
}
