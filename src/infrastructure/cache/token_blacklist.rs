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

    /// Revoke a token until `ttl_seconds` elapses (set to JWT remaining lifetime).
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
}
