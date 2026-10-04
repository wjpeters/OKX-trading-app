//! Encrypted at rest; key file is separate from the database and never available to Pine/UI.
use anyhow::{Result, ensure};
use ring::aead;
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
pub async fn read(purpose: &str) -> Result<String> {
    ensure!(
        ["openrouter_key", "okx_demo"].contains(&purpose),
        "Secret purpose niet toegestaan"
    );
    let envelope = tokio::fs::read(format!("/run/secrets/{purpose}")).await?;
    let key = tokio::fs::read("/run/secrets/local_secrets_key").await?;
    ensure!(envelope.len() <= 8192, "Secret-envelope te groot");
    Ok(String::from_utf8(decrypt(&envelope, &key, purpose)?)?)
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
    fn plaintext_credentials_rejected() {
        assert!(decrypt(b"not-encrypted", &[0; 32], "openrouter_key").is_err());
    }
}
