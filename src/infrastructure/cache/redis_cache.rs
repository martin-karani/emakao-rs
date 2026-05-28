use crate::application::errors::AppError;
use fred::{clients::RedisPool, interfaces::KeysInterface, prelude::*};
use std::time::Duration;

#[derive(Clone)]
pub struct RedisCache {
    pool: RedisPool,
}

impl RedisCache {
    /// Build from an already-connected pool (handed in from AppState).
    /// No connect() call needed — the pool is already live.
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    pub async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), AppError> {
        let expiry = ttl.map(|d| Expiration::EX(d.as_secs() as i64));
        self.pool
            .set::<(), _, _>(key, value, expiry, None, false)
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        self.pool
            .get::<Option<String>, _>(key)
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))
    }

    pub async fn del(&self, key: &str) -> Result<(), AppError> {
        self.pool
            .del::<i64, _>(key)
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?;
        Ok(())
    }

    pub async fn ping(&self) -> Result<String, AppError> {
        self.pool
            .ping::<String>()
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))
    }
    // NOTE: Token revocation is handled exclusively by `TokenBlacklist`
    // (key prefix `blacklist:{jti}`).  Do NOT add duplicate revoke_token /
    // is_token_revoked methods here — they would use a different prefix and
    // silently never be checked by the auth middleware.  See ISSUE 7.
}
