//!
//! Thin Redis cache layer for subscription state and entitlements.
//! Mirrors the NestJS CacheService usage in SubscriptionService.
//!
//! TTLs:
//!   subscription:state:*  → 300 s
//!   entitlements:*        → 300 s
//!   usage:sms:*           → current calendar month (TTL = seconds until midnight)
//!   usage:wa:*            → current calendar month
//!   usage:api:*           → until end of day
//!   usage:storage:*       → no TTL (reconciled from S3/DB)

use fred::{clients::RedisPool, interfaces::KeysInterface};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::subscription::{AgencyEntitlements, LimitKey, SubscriptionState};

const STATE_TTL: u32 = 300;
const ENTITLE_TTL: u32 = 300;

pub struct SubscriptionCache {
    redis: RedisPool,
}

impl SubscriptionCache {
    pub fn new(redis: RedisPool) -> Self {
        Self { redis }
    }

    //  State ─

    pub async fn get_state(&self, agency_id: Uuid) -> Option<SubscriptionState> {
        let key = format!("subscription:state:{agency_id}");
        let raw: Option<String> = self.redis.get(&key).await.ok()?;
        raw.and_then(|s| serde_json::from_str(&s).ok())
    }

    pub async fn set_state(&self, agency_id: Uuid, state: &SubscriptionState) {
        let key = format!("subscription:state:{agency_id}");
        if let Ok(json) = serde_json::to_string(state) {
            let _: Result<(), _> = self
                .redis
                .set(
                    &key,
                    json,
                    Some(fred::types::Expiration::EX(STATE_TTL as i64)),
                    None,
                    false,
                )
                .await;
        }
    }

    //  Entitlements

    pub async fn get_entitlements(&self, agency_id: Uuid) -> Option<AgencyEntitlements> {
        let key = format!("entitlements:{agency_id}");
        let raw: Option<String> = self.redis.get(&key).await.ok()?;
        raw.and_then(|s| serde_json::from_str(&s).ok())
    }

    pub async fn set_entitlements(&self, agency_id: Uuid, e: &AgencyEntitlements) {
        let key = format!("entitlements:{agency_id}");
        if let Ok(json) = serde_json::to_string(e) {
            let _: Result<(), _> = self
                .redis
                .set(
                    &key,
                    json,
                    Some(fred::types::Expiration::EX(ENTITLE_TTL as i64)),
                    None,
                    false,
                )
                .await;
        }
    }

    //  Invalidation (call on plan change / cancellation)

    pub async fn invalidate(&self, agency_id: Uuid) {
        let state_key = format!("subscription:state:{agency_id}");
        let entitle_key = format!("entitlements:{agency_id}");
        let _: Result<(), _> = self.redis.del::<(), _>(&[state_key, entitle_key]).await;
        
        // Also publish to the cross-pod invalidation channel for in-memory caches
        // (SettingsCache, EntitlementCache).
        use fred::interfaces::PubsubInterface;
        let _: Result<(), _> = self
            .redis
            .next()
            .publish::<(), _, _>("cache:invalidate:agency", agency_id.to_string())
            .await;
    }

    //  Usage counters (monthly SMS / WhatsApp, daily API calls)

    /// Increment a monthly usage counter (e.g. SMS sent this month).
    /// Key: usage:{agency_id}:sms:2025-06
    /// TTL is set to expire at end of the billing month.
    pub async fn increment_monthly_counter(&self, agency_id: Uuid, key: LimitKey, by: i64) {
        let month = current_month();
        let field = format!("usage:{}:{}:{}", agency_id, key.as_str(), month);
        let _: Result<(), _> = self.redis.incr_by::<(), _>(&field, by).await;
        // Set TTL to end of this month (so counters reset automatically)
        let ttl = seconds_until_end_of_month() as i64;
        let _: Result<(), _> = self.redis.expire::<(), _>(&field, ttl).await;
    }

    pub async fn get_monthly_counter(&self, agency_id: Uuid, key: LimitKey) -> i32 {
        let month = current_month();
        let field = format!("usage:{}:{}:{}", agency_id, key.as_str(), month);
        let val: Option<i64> = self.redis.get(&field).await.ok().flatten();
        val.unwrap_or(0) as i32
    }

    /// Increment a daily counter (e.g. API calls today).
    pub async fn increment_daily_counter(&self, agency_id: Uuid, key: LimitKey, by: i64) {
        let today = current_date();
        let field = format!("usage:{}:{}:{}", agency_id, key.as_str(), today);
        let _: Result<(), _> = self.redis.incr_by::<(), _>(&field, by).await;
        let ttl = seconds_until_midnight() as i64;
        let _: Result<(), _> = self.redis.expire::<(), _>(&field, ttl).await;
    }

    pub async fn get_daily_counter(&self, agency_id: Uuid, key: LimitKey) -> i32 {
        let today = current_date();
        let field = format!("usage:{}:{}:{}", agency_id, key.as_str(), today);
        let val: Option<i64> = self.redis.get(&field).await.ok().flatten();
        val.unwrap_or(0) as i32
    }

    /// Storage bytes (no TTL — reconciled from DB/S3).
    pub async fn get_usage_bytes(&self, agency_id: Uuid) -> i64 {
        let key = format!("usage:{}:storage_bytes", agency_id);
        let val: Option<i64> = self.redis.get(&key).await.ok().flatten();
        val.unwrap_or(0)
    }

    pub async fn set_usage_bytes(&self, agency_id: Uuid, bytes: i64) {
        let key = format!("usage:{}:storage_bytes", agency_id);
        let _: Result<(), _> = self
            .redis
            .set::<(), _, _>(&key, bytes, None, None, false)
            .await;
    }
}

//  Time helpers

fn current_month() -> String {
    let now = OffsetDateTime::now_utc();
    format!("{}-{:02}", now.year(), now.month() as u8)
}

fn current_date() -> String {
    let now = OffsetDateTime::now_utc();
    format!("{}-{:02}-{:02}", now.year(), now.month() as u8, now.day())
}

fn seconds_until_end_of_month() -> u64 {
    use time::{Date, Month};
    let now = OffsetDateTime::now_utc();
    let year = now.year();
    let mon = now.month();
    // First day of next month
    let (next_y, next_m) = if mon == Month::December {
        (year + 1, Month::January)
    } else {
        (year, mon.next())
    };
    let next_month_start = Date::from_calendar_date(next_y, next_m, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    (next_month_start - now).whole_seconds().max(0) as u64
}

fn seconds_until_midnight() -> u64 {
    let now = OffsetDateTime::now_utc();
    let tomorrow = (now + Duration::days(1)).date().midnight().assume_utc();
    (tomorrow - now).whole_seconds().max(0) as u64
}
