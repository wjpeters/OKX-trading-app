//! Encrypted at rest; key file is separate from the database and never available to Pine/UI.
use anyhow::{Result, ensure};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use std::{io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};
pub fn decrypt(envelope: &[u8], key: &[u8], purpose: &str) -> Result<Vec<u8>> {
    ensure!(
        key.len() == 32 && envelope.len() >= 33 && &envelope[..5] == b"OKXS1",
        "Secret-envelope ongeldig; voer de lokale sleutelsetup opnieuw uit"
    );
    let nonce: [u8; 12] = envelope[5..17].try_into()?;
    let key = aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|_| anyhow::anyhow!("Secret key ongeldig"))?,
    );
    let mut body = envelope[17..].to_vec();
    let plain = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(purpose.as_bytes()),
            &mut body,
        )
        .map_err(|_| anyhow::anyhow!("Secret-integriteitscontrole mislukt"))?;
    Ok(plain.to_vec())
}
fn allowed(purpose: &str) -> bool {
    ["openrouter_key", "typesafe_key", "okx_demo", "okx_live"].contains(&purpose)
}
fn path(purpose: &str) -> Result<PathBuf> {
    ensure!(allowed(purpose), "Secret purpose niet toegestaan");
    Ok(
        PathBuf::from(std::env::var("SETTINGS_DIR").unwrap_or_else(|_| "/run/secrets".into()))
            .join(purpose),
    )
}
pub async fn read(purpose: &str) -> Result<String> {
    let path = path(purpose)?;
    ensure!(
        tokio::fs::metadata(&path).await?.len() <= 8192,
        "Secret-envelope te groot"
    );
    let envelope = tokio::fs::read(path).await?;
    let key = tokio::fs::read("/run/secrets/local_secrets_key").await?;
    Ok(String::from_utf8(decrypt(&envelope, &key, purpose)?)?)
}
pub async fn configured(purpose: &str) -> bool {
    read(purpose).await.is_ok_and(|s| !s.trim().is_empty())
}
pub fn encrypt(plain: &[u8], key: &[u8], purpose: &str) -> Result<Vec<u8>> {
    ensure!(allowed(purpose) && plain.len() <= 7000, "Secret ongeldig");
    let key = aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|_| anyhow::anyhow!("Secret key ongeldig"))?,
    );
    let mut nonce = [0u8; 12];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| anyhow::anyhow!("Randombron niet beschikbaar"))?;
    let mut body = plain.to_vec();
    key.seal_in_place_append_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::from(purpose.as_bytes()),
        &mut body,
    )
    .map_err(|_| anyhow::anyhow!("Versleutelen mislukt"))?;
    let mut envelope = b"OKXS1".to_vec();
    envelope.extend(nonce);
    envelope.extend(body);
    Ok(envelope)
}
// Atomic replacement, restrictive modes, no plaintext ever written to disk.
// Empty encrypted values are deletion tombstones: old credentials cannot reappear.
pub async fn write(purpose: &str, value: &str) -> Result<()> {
    ensure!(
        std::env::var("SETTINGS_DIR").is_ok(),
        "Start de terminal opnieuw via het startbestand om sleutelbeheer te activeren"
    );
    let path = path(purpose)?;
    let key = tokio::fs::read("/run/secrets/local_secrets_key").await?;
    let envelope = encrypt(value.as_bytes(), &key, purpose)?;
    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut suffix = [0u8; 8];
        SystemRandom::new()
            .fill(&mut suffix)
            .map_err(|_| anyhow::anyhow!("Randombron niet beschikbaar"))?;
        let tmp = path.with_extension(format!("{:016x}.writing", u64::from_le_bytes(suffix)));
        // All writes are serialized by the provider/settings database lease.
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)?;
        let result = (|| -> Result<()> {
            f.write_all(&envelope)?;
            f.sync_all()?;
            std::fs::rename(&tmp, &path)?;
            std::fs::File::open(path.parent().unwrap())?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    })
    .await??;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_fixture_is_authenticated_and_scoped() {
        let key = [3u8; 32];
        let nonce = [7u8; 12];
        let mut payload = b"fixture-only".to_vec();
        let seal = aead::LessSafeKey::new(aead::UnboundKey::new(&aead::AES_256_GCM, &key).unwrap());
        seal.seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(b"openrouter_key"),
            &mut payload,
        )
        .unwrap();
        let mut env = b"OKXS1".to_vec();
        env.extend(nonce);
        env.extend(payload);
        assert_eq!(
            decrypt(&env, &key, "openrouter_key").unwrap(),
            b"fixture-only"
        );
        assert!(decrypt(&env, &key, "okx_demo").is_err());
        *env.last_mut().unwrap() ^= 1;
        assert!(decrypt(&env, &key, "openrouter_key").is_err());
    }
    #[test]
    fn fresh_encryption_uses_unique_nonces_and_scope() {
        let a = encrypt(b"fixture", &[3; 32], "typesafe_key").unwrap();
        let b = encrypt(b"fixture", &[3; 32], "typesafe_key").unwrap();
        assert_ne!(a, b);
        assert_eq!(decrypt(&a, &[3; 32], "typesafe_key").unwrap(), b"fixture");
        assert!(decrypt(&a, &[3; 32], "openrouter_key").is_err());
        assert!(encrypt(b"fixture", &[3; 32], "../escape").is_err());
    }
    #[test]
    fn plaintext_credentials_rejected() {
        assert!(decrypt(b"not-encrypted", &[0; 32], "openrouter_key").is_err());
    }
}
