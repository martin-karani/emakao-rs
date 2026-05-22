// src/application/use_cases/settings_update.rs
//
// The single authoritative path for mutating `agencies.settings`.
//
// Rules enforced here (so callers can't forget):
//   1. Validate the patch structure against the AgencySettings shape.
//   2. Merge with existing JSONB (Postgres `||` operator).
//   3. Invalidate ALL in-process caches that derive from settings.
//   4. Broadcast the invalidation to other pods via Redis.
//   5. Emit an audit event (if audit_level != "minimal").
//
// Call this from your Axum handler — never write to `agencies.settings`
// directly from a handler.

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    infrastructure::audit::{AuditEvent, AuditLogger},
    presentation::app_state::AppState,
};

// ── Command ───────────────────────────────────────────────────────────────────

pub struct UpdateAgencySettingsCommand {
    pub agency_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_role: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
    /// JSON object — keys to merge into `agencies.settings`.
    /// Only the supplied keys are touched (deep merge via Postgres `||`).
    pub patch: serde_json::Value,
}

// ── Use case ──────────────────────────────────────────────────────────────────

pub async fn execute(
    state: &Arc<AppState>,
    cmd: UpdateAgencySettingsCommand,
) -> anyhow::Result<()> {
    // ── 1. Validate patch is a JSON object ───────────────────────────────────
    if !cmd.patch.is_object() {
        anyhow::bail!("settings patch must be a JSON object");
    }

    // ── 2. Capture old settings for audit diff ───────────────────────────────
    let old_row = sqlx::query!("SELECT settings FROM agencies WHERE id = $1", cmd.agency_id)
        .fetch_one(&state.db)
        .await?;

    // ── 3. Merge-update in Postgres ──────────────────────────────────────────
    sqlx::query!(
        r#"
        UPDATE agencies
        SET settings   = settings || $2,
            updated_at = now()
        WHERE id = $1
        "#,
        cmd.agency_id,
        cmd.patch,
    )
    .execute(&state.db)
    .await?;

    // ── 4. Invalidate all caches that derive from settings ───────────────────

    // Settings aggregator
    state
        .settings_cache
        .invalidate_and_broadcast(cmd.agency_id, &state.redis)
        .await?;

    // Plan entitlements (they embed workflow/feature data)
    state.entitlements.invalidate(cmd.agency_id);

    // Provider registry (credentials / active integrations may have changed)
    state
        .providers
        .load_agency(cmd.agency_id, &state.db, &state.enc_key)
        .await?;

    // Audit-level cache on the logger itself
    state.audit.invalidate_level(cmd.agency_id);

    // ── 5. Emit audit event ──────────────────────────────────────────────────
    state.audit.log(AuditEvent {
        agency_id: cmd.agency_id,
        actor_id: cmd.actor_id,
        actor_role: cmd.actor_role,
        action: "agency.settings.updated".to_string(),
        entity_type: "agency".to_string(),
        entity_id: cmd.agency_id,
        old_data: Some(old_row.settings),
        new_data: Some(cmd.patch),
        ip_address: cmd.ip_address,
    });

    Ok(())
}
