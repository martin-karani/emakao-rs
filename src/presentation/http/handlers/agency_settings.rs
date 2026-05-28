// src/presentation/http/handlers/agency_settings.rs
//
// Two handlers:
//   GET  /api/agency/settings          — return the full AgencySettings for the agency
//   PATCH /api/agency/settings         — merge-update settings, invalidate all caches
//
// Both require `require_auth` + `resolve_agency_context` to have run first
// so that `AuthenticatedUser` and `ResolvedAgency` are in extensions.
//
// Access control: only staff with the "agency_settings:write" permission may PATCH.

use std::sync::Arc;

use axum::{
    extract::{Extension, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    application::{
        errors::AppError,
        use_cases::settings_update::{self, UpdateAgencySettingsCommand},
    },
    domain::{agency_settings::AgencySettings, auth::AuthenticatedUser},
    presentation::app_state::AppState,
};

// ── GET /api/agency/settings ──────────────────────────────────────────────────

/// Return the full `AgencySettings` for the authenticated agency.
/// Settings are served from the DashMap cache; first call loads from Postgres.
#[utoipa::path(
    get,
    path = "/api/agency/settings",
    tag  = "Agency",
    responses(
        (status = 200, description = "Agency settings", body = AgencySettingsResponse),
        (status = 401, description = "Unauthorised"),
        (status = 500, description = "Internal error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let settings = state
        .customisation()
        .settings
        .get_or_load(user.agency_id, state.infra.tenant_pools.platform())
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    Ok(Json(AgencySettingsResponse::from(settings.as_ref())))
}

// ── PATCH /api/agency/settings ────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchSettingsRequest {
    /// A JSON object whose keys are merged into `agencies.settings`.
    /// Only supplied top-level keys are touched — unspecified keys are preserved.
    pub patch: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PatchSettingsResponse {
    pub message: String,
}

/// Merge-update the agency's settings.
/// Invalidates the settings cache, entitlement cache, provider registry,
/// and broadcasts the invalidation to all pods via Redis pub/sub.
#[utoipa::path(
    patch,
    path = "/api/agency/settings",
    tag  = "Agency",
    request_body = PatchSettingsRequest,
    responses(
        (status = 200, description = "Settings updated"),
        (status = 400, description = "Invalid patch (must be a JSON object)"),
        (status = 401, description = "Unauthorised"),
        (status = 403, description = "Forbidden — requires agency_settings:write"),
        (status = 500, description = "Internal error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn patch_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(body): Json<PatchSettingsRequest>,
) -> Result<impl IntoResponse, AppError> {
    // ── Permission check ──────────────────────────────────────────────────────
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    // ── Execute use-case ──────────────────────────────────────────────────────
    settings_update::execute(
        &Arc::new(state.clone()),
        UpdateAgencySettingsCommand {
            agency_id: user.agency_id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            ip_address: None, // TODO: extract from ConnectInfo extension
            patch: body.patch,
        },
    )
    .await
    .map_err(|e| AppError::InternalServer(e.to_string()))?;

    Ok(Json(PatchSettingsResponse {
        message: "Settings updated".into(),
    }))
}

// ── Response DTO ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct AgencySettingsResponse {
    pub branding: serde_json::Value,
    pub communication: serde_json::Value,
    pub workflows: serde_json::Value,
    pub ui: serde_json::Value,
    pub portal: serde_json::Value,
    pub onboarding: serde_json::Value,
    pub extra: serde_json::Value,
}

impl From<&AgencySettings> for AgencySettingsResponse {
    fn from(s: &AgencySettings) -> Self {
        // Round-trip through serde so we get camelCase/snake_case consistency.
        let v = serde_json::to_value(s).unwrap_or_default();
        Self {
            branding: v["branding"].clone(),
            communication: v["communication"].clone(),
            workflows: v["workflows"].clone(),
            ui: v["ui"].clone(),
            portal: v["portal"].clone(),
            onboarding: v["onboarding"].clone(),
            extra: v["extra"].clone(),
        }
    }
}
