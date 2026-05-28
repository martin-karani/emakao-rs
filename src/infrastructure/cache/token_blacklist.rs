use anyhow::Result;
use fred::clients::RedisPool;
use fred::interfaces::KeysInterface;

pub struct TokenBlacklist {
    redis: RedisPool,
}

impl TokenBlacklist {
    pub fn new(redis: RedisPool) -> Self {
        Self { redis }
    }

    /// Returns `true` if the given JWT ID has been revoked (logged out).
    pub async fn is_revoked(&self, jti: &str) -> Result<bool> {
        let key = format!("blacklist:{jti}");
        let exists: Option<String> = self.redis.get(&key).await?;
        Ok(exists.is_some())
    }

    pub async fn revoke(&self, jti: &str, ttl_seconds: i64) -> Result<()> {
        let key = format!("blacklist:{jti}");
        let _: () = self
            .redis
            .set(
                &key,
                "1",
                Some(fred::types::Expiration::EX(ttl_seconds)),
                None,
                false,
            )
            .await?;
        Ok(())
    }

    /// Check if the user has changed their password after the given timestamp
    pub async fn is_revoked_by_iat(&self, user_id: uuid::Uuid, iat: usize) -> Result<bool> {
        let key = format!("user:{}:password_changed_at", user_id);
        let changed_at: Option<String> = self.redis.get(&key).await?;
        if let Some(changed_str) = changed_at {
            if let Ok(changed) = changed_str.parse::<usize>() {
                // Token is revoked if issued before or at the exact same second as password change
                return Ok(iat <= changed);
            }
        }
        Ok(false)
    }

    /// Revoke all tokens issued before `timestamp`. Stores the timestamp for the max JWT lifetime.
    pub async fn revoke_all_before(&self, user_id: uuid::Uuid, timestamp: usize, max_ttl_seconds: i64) -> Result<()> {
        let key = format!("user:{}:password_changed_at", user_id);
        let _: () = self
            .redis
            .set(
                &key,
                timestamp.to_string(),
                Some(fred::types::Expiration::EX(max_ttl_seconds)),
                None,
                false,
            )
            .await?;
        Ok(())
    }
}
