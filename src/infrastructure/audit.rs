// AuditLogger — non-blocking INSERT to the partitioned `audit_log` table.
//
// The hot path never waits on the audit INSERT (tokio::spawn).
// audit_level controls what gets logged:
//   "minimal"   → only login / authentication events
//   "financial"  → all payment and financial events  (default)
//   "full"       → every field-level change

use dashmap::DashMap;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug)]
pub struct AuditEvent {
    pub agency_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_role: Option<String>,
    /// Dot-separated path, e.g. "agreement.rent_amount.updated"
    pub action: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub old_data: Option<serde_json::Value>,
    pub new_data: Option<serde_json::Value>,
    pub ip_address: Option<std::net::IpAddr>,
}

pub struct AuditLogger {
    pool: PgPool,
    /// Cache of audit_level per agency.  Avoids a DB hit on every event.
    levels: DashMap<Uuid, String>,
}

impl AuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            levels: DashMap::new(),
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Queue an audit event.  Returns immediately; the INSERT runs in the
    /// background.  Any INSERT failure is logged but does NOT propagate.
    pub fn log(&self, event: AuditEvent) {
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let ip = event.ip_address.map(|ip| ip.to_string());
            if let Err(e) = sqlx::query(
                r#"
                INSERT INTO audit_log
                    (agency_id, actor_id, actor_role, action,
                     entity_type, entity_id, old_data, new_data, ip_address)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(event.agency_id)
            .bind(event.actor_id)
            .bind(event.actor_role)
            .bind(event.action)
            .bind(event.entity_type)
            .bind(event.entity_id)
            .bind(event.old_data)
            .bind(event.new_data)
            .bind(ip)
            .execute(&pool)
            .await
            {
                tracing::error!("audit_log INSERT failed: {e}");
            }
        });
    }

    /// Return the audit level for an agency, loading from Postgres if not cached.
    pub async fn level(&self, agency_id: Uuid) -> String {
        if let Some(l) = self.levels.get(&agency_id) {
            return l.clone();
        }
        match sqlx::query_scalar::<_, String>("SELECT audit_level FROM agencies WHERE id = $1")
            .bind(agency_id)
            .fetch_one(&self.pool)
            .await
        {
            Ok(level) => {
                self.levels.insert(agency_id, level.clone());
                level
            }
            Err(e) => {
                tracing::error!("audit_logger: failed to load level for {agency_id}: {e}");
                "financial".to_string()
            }
        }
    }

    /// `true` only when audit_level = "full" — required before logging
    /// individual field-level diffs.
    pub async fn should_log_field_changes(&self, agency_id: Uuid) -> bool {
        self.level(agency_id).await == "full"
    }

    /// Call after any mutation to `agencies.audit_level`.
    pub fn invalidate_level(&self, agency_id: Uuid) {
        self.levels.remove(&agency_id);
    }
}
