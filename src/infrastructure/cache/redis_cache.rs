use crate::application::errors::AppError;
use fred::{clients::RedisPool, interfaces::KeysInterface, prelude::*};
use std::time::Duration;

#[derive(Clone)]
pub struct RedisCache {
    pool: RedisPool, // ← borrows from AppState's pool, no new connection
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

    pub async fn revoke_token(&self, jti: &str, ttl: Duration) -> Result<(), AppError> {
        self.set(&format!("revoked:{jti}"), "1", Some(ttl)).await
    }

    pub async fn is_token_revoked(&self, jti: &str) -> Result<bool, AppError> {
        Ok(self.get(&format!("revoked:{jti}")).await?.is_some())
    }
}
