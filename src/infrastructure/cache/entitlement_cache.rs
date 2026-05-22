// src/infrastructure/cache/entitlement_cache.rs
//
// Resolves the merged set of plan features + per-agency overrides for a
// given agency, caches the result in a DashMap, and exposes typed helpers
// (`is_enabled`, `numeric_limit`) used by the Axum macros in
// src/presentation/macros.rs.

use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub type FeatureMap = serde_json::Map<String, Value>;

pub struct EntitlementCache {
    pool: PgPool,
    inner: DashMap<Uuid, FeatureMap>,
}

impl EntitlementCache {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            inner: DashMap::new(),
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Plan features merged with live per-agency overrides. Cached until
    /// invalidated (call `invalidate` after any plan or override mutation).
    pub async fn resolve(&self, agency_id: Uuid) -> anyhow::Result<FeatureMap> {
        if let Some(entry) = self.inner.get(&agency_id) {
            return Ok(entry.clone());
        }

        let row = sqlx::query!(
            r#"
            SELECT
                sp.features AS plan_features,
                COALESCE(
                    jsonb_object_agg(apo.feature_key, apo.value)
                    FILTER (
                        WHERE apo.id IS NOT NULL
                          AND (apo.expires_at IS NULL OR apo.expires_at > now())
                    ),
                    '{}'::jsonb
                ) AS "overrides!"
            FROM agency_subscriptions asub
            JOIN subscription_plans sp ON sp.id = asub.plan_id
            LEFT JOIN agency_plan_overrides apo
                   ON apo.agency_id = asub.agency_id
            WHERE asub.agency_id = $1
            GROUP BY sp.features
            "#,
            agency_id
        )
        .fetch_one(&self.pool)
        .await?;

        let mut merged: FeatureMap = row.plan_features.as_object().cloned().unwrap_or_default();

        if let Some(ov) = row.overrides.as_object() {
            merged.extend(ov.iter().map(|(k, v)| (k.clone(), v.clone())));
        }

        self.inner.insert(agency_id, merged.clone());
        Ok(merged)
    }

    /// Returns `true` if the boolean feature flag is enabled for this agency.
    pub async fn is_enabled(&self, agency_id: Uuid, key: &str) -> bool {
        self.resolve(agency_id)
            .await
            .ok()
            .and_then(|m| m.get(key)?.as_bool())
            .unwrap_or(false)
    }

    /// Returns the numeric limit for a feature key, or `None` if unlimited.
    pub async fn numeric_limit(&self, agency_id: Uuid, key: &str) -> Option<i64> {
        self.resolve(agency_id)
            .await
            .ok()
            .and_then(|m| m.get(key)?.as_i64())
    }

    /// Drop the cached entry; the next call to `resolve` reloads from Postgres.
    pub fn invalidate(&self, agency_id: Uuid) {
        self.inner.remove(&agency_id);
    }
}
