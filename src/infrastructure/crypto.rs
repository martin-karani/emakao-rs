// src/infrastructure/crypto.rs
//
// AES-256-GCM encrypt / decrypt for `agency_integrations.credentials`.
//
// The ciphertext is stored as a JSONB object:
//   { "iv": "<base64>", "ct": "<base64>" }
//
// Dependencies to add to Cargo.toml:
//   aes-gcm = "0.10"
//   (base64 is already present)

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

const NONCE_LEN: usize = 12; // 96-bit nonce — GCM standard

// ── Encrypt ───────────────────────────────────────────────────────────────────

/// Encrypt a JSON value and return a `{ "iv": "...", "ct": "..." }` JSONB
/// object suitable for storage in `agency_integrations.credentials`.
pub fn encrypt_jsonb(
    plaintext: &serde_json::Value,
    key: &[u8; 32],
) -> anyhow::Result<serde_json::Value> {
    let raw = serde_json::to_vec(plaintext)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, raw.as_ref())
        .map_err(|e| anyhow::anyhow!("AES-GCM encrypt failed: {e}"))?;

    Ok(serde_json::json!({
        "iv": B64.encode(nonce_bytes),
        "ct": B64.encode(ciphertext),
    }))
}

// ── Decrypt ───────────────────────────────────────────────────────────────────

/// Decrypt a `{ "iv": "...", "ct": "..." }` JSONB value produced by
/// `encrypt_jsonb`.  Returns the original JSON value on success.
pub fn decrypt_jsonb(
    blob: &serde_json::Value,
    key: &[u8; 32],
) -> anyhow::Result<serde_json::Value> {
    let iv_b64 = blob
        .get("iv")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("credentials blob missing 'iv'"))?;
    let ct_b64 = blob
        .get("ct")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("credentials blob missing 'ct'"))?;

    let nonce_bytes = B64.decode(iv_b64)?;
    let ciphertext = B64.decode(ct_b64)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("AES-GCM decrypt failed: {e}"))?;

    Ok(serde_json::from_slice(&plaintext)?)
}

// ── Key loading ───────────────────────────────────────────────────────────────

/// Load the 32-byte AES key from the `CREDENTIALS_ENC_KEY` environment
/// variable (hex-encoded, 64 chars).  Call once at startup.
///
/// Generate a key: `openssl rand -hex 32`
pub fn load_enc_key() -> anyhow::Result<[u8; 32]> {
    let hex = std::env::var("CREDENTIALS_ENC_KEY")
        .map_err(|_| anyhow::anyhow!("CREDENTIALS_ENC_KEY env var is required"))?;
    let bytes = hex::decode(&hex)
        .map_err(|e| anyhow::anyhow!("CREDENTIALS_ENC_KEY is not valid hex: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("CREDENTIALS_ENC_KEY must be exactly 32 bytes (64 hex chars)"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let key: [u8; 32] = rand::random();
        let plain = serde_json::json!({
            "api_key": "secret-123",
            "username": "sandbox",
        });
        let blob = encrypt_jsonb(&plain, &key).unwrap();
        let recovered = decrypt_jsonb(&blob, &key).unwrap();
        assert_eq!(plain, recovered);
    }

    #[test]
    fn wrong_key_fails() {
        let key1: [u8; 32] = rand::random();
        let key2: [u8; 32] = rand::random();
        let plain = serde_json::json!({ "x": 1 });
        let blob = encrypt_jsonb(&plain, &key1).unwrap();
        assert!(decrypt_jsonb(&blob, &key2).is_err());
    }
}
