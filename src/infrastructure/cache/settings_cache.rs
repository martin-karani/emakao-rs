use std::sync::Arc;

use dashmap::DashMap;
use fred::clients::SubscriberClient;
use fred::interfaces::{EventInterface, PubsubInterface};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::agency_settings::AgencySettings;

pub struct SettingsCache {
    inner: DashMap<Uuid, Arc<AgencySettings>>,
}

impl Default for SettingsCache {
    fn default() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }
}

impl SettingsCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return cached settings, or load from Postgres and cache the result.
    pub async fn get_or_load(
        &self,
        agency_id: Uuid,
        pool: &PgPool,
    ) -> anyhow::Result<Arc<AgencySettings>> {
        if let Some(entry) = self.inner.get(&agency_id) {
            return Ok(Arc::clone(&entry));
        }

        let row = sqlx::query!(r#"SELECT settings FROM agencies WHERE id = $1"#, agency_id)
            .fetch_one(pool)
            .await?;

        let settings = Arc::new(AgencySettings::from_jsonb(&row.settings));
        self.inner.insert(agency_id, Arc::clone(&settings));
        Ok(settings)
    }

    /// Remove the cached entry so the next call reloads from Postgres.
    pub fn invalidate(&self, agency_id: Uuid) {
        self.inner.remove(&agency_id);
    }

    /// Invalidate and publish to the Redis channel so every other pod
    /// also drops its copy.
    pub async fn invalidate_and_broadcast(
        &self,
        agency_id: Uuid,
        redis: &fred::clients::RedisPool,
    ) -> anyhow::Result<()> {
        self.invalidate(agency_id);
        redis.next()
            .publish::<(), _, _>("cache:invalidate:agency", agency_id.to_string())
            .await?;
        Ok(())
    }
}

// ── Listener (run once at startup) ───────────────────────────────────────────

/// Subscribe to the Redis invalidation channel and drop local entries as
/// messages arrive from other pods.
///
/// Call with `tokio::spawn(run_invalidation_listener(subscriber, cache))`.
pub async fn run_invalidation_listener(subscriber: SubscriberClient, cache: Arc<SettingsCache>) {
    if let Err(e) = subscriber
        .subscribe("cache:invalidate:agency")
        .await
    {
        tracing::error!("settings cache: failed to subscribe: {e}");
        return;
    }

    let mut rx = subscriber.message_rx();

    loop {
        match rx.recv().await {
            Ok(msg) => {
                if let Some(id_str) = msg.value.as_str() {
                    match Uuid::parse_str(&id_str) {
                        Ok(id) => {
                            cache.invalidate(id);
                            tracing::debug!("settings cache: invalidated {id}");
                        }
                        Err(e) => {
                            tracing::warn!("settings cache: bad UUID in pub/sub message: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("settings cache: recv error: {e}");
                break;
            }
        }
    }
}
