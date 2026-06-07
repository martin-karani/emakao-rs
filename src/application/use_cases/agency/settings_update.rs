use std::sync::Arc;

use uuid::Uuid;

use crate::{infrastructure::audit::AuditEvent, presentation::app_state::AppState};
use sqlx::Row;

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
    let old_settings: serde_json::Value =
        sqlx::query("SELECT settings FROM agencies WHERE id = $1")
            .bind(cmd.agency_id)
            .fetch_one(state.infra.tenant_pools.platform())
            .await?
            .try_get("settings")?;

    // ── 3. Merge-update in Postgres ──────────────────────────────────────────
    sqlx::query(
        r#"
        UPDATE agencies
        SET settings   = settings || $2,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(cmd.agency_id)
    .bind(cmd.patch.clone())
    .execute(state.infra.tenant_pools.platform())
    .await?;

    // ── 4. Invalidate all caches that derive from settings ───────────────────

    // Settings aggregator
    state
        .customisation()
        .settings
        .invalidate_and_broadcast(cmd.agency_id, &state.infra.redis)
        .await?;

    // Plan entitlements (they embed workflow/feature data)
    state.customisation().entitlements.invalidate(cmd.agency_id);

    // Provider registry (credentials / active integrations may have changed)
    let agency_pool = state.infra.tenant_pools.for_agency(cmd.agency_id).await?;
    state
        .customisation()
        .providers
        .load_agency(cmd.agency_id, &agency_pool, &*state.customisation().enc_key)
        .await?;

    // Audit-level cache on the logger itself
    state.customisation().audit.invalidate_level(cmd.agency_id);

    // ── 5. Emit audit event ──────────────────────────────────────────────────
    state.customisation().audit.log(AuditEvent {
        agency_id: cmd.agency_id,
        actor_id: cmd.actor_id,
        actor_role: cmd.actor_role,
        action: "agency.settings.updated".to_string(),
        entity_type: "agency".to_string(),
        entity_id: cmd.agency_id,
        old_data: Some(old_settings),
        new_data: Some(cmd.patch.clone()),
        ip_address: cmd.ip_address,
    });

    Ok(())
}
