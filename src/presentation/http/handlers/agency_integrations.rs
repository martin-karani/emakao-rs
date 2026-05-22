// src/presentation/http/handlers/agency_integrations.rs
//
// Handlers for per-agency provider integrations stored in
// `agency_integrations`.
//
//   GET    /api/agency/integrations              — list active integrations (credentials redacted)
//   PUT    /api/agency/integrations/:type/:key   — upsert credentials + settings
//   DELETE /api/agency/integrations/:type/:key   — deactivate
//
// All handlers require "agency_settings:write".
// Credentials are AES-256-GCM encrypted before INSERT/UPDATE.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    application::errors::AppError, domain::auth::AuthenticatedUser,
    infrastructure::crypto::encrypt_jsonb, presentation::app_state::AppState,
};

// ── GET /api/agency/integrations ─────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct IntegrationSummary {
    pub provider_type: String,
    pub provider_key: String,
    pub is_active: bool,
    /// Non-secret settings (shortcodes, bucket names) returned as-is.
    pub settings: serde_json::Value,
    /// Credentials are never returned — redacted to protect secrets.
    pub credentials: &'static str,
}

pub async fn list_integrations(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    let rows = sqlx::query!(
        r#"
        SELECT provider_type, provider_key, is_active, settings
        FROM   agency_integrations
        WHERE  agency_id = $1
        ORDER  BY provider_type, provider_key
        "#,
        user.agency_id,
    )
    .fetch_all(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    let items: Vec<IntegrationSummary> = rows
        .into_iter()
        .map(|r| IntegrationSummary {
            provider_type: r.provider_type,
            provider_key: r.provider_key,
            is_active: r.is_active,
            settings: r.settings,
            credentials: "[redacted]",
        })
        .collect();

    Ok(Json(items))
}

// ── PUT /api/agency/integrations/:type/:key ───────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertIntegrationRequest {
    /// Plain-text credentials — will be encrypted at rest.
    /// Shape depends on provider:
    ///   mpesa:            { consumer_key, consumer_secret, passkey }
    ///   africas_talking:  { api_key, username, sender_id? }
    ///   sendgrid:         { api_key, from_email?, from_name? }
    ///   twilio:           { account_sid, auth_token, from_number }
    pub credentials: serde_json::Value,
    /// Non-secret config (shortcodes, callback URLs, bucket names).
    #[serde(default)]
    pub settings: serde_json::Value,
}

pub async fn upsert_integration(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((provider_type, provider_key)): Path<(String, String)>,
    Json(body): Json<UpsertIntegrationRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    // Encrypt credentials before storing.
    let enc_key = &*state.custom().enc_key;
    let encrypted = encrypt_jsonb(&body.credentials, enc_key)
        .map_err(|e| AppError::InternalServer(format!("encryption failed: {e}")))?;

    sqlx::query!(
        r#"
        INSERT INTO agency_integrations
            (agency_id, provider_type, provider_key, credentials, settings, is_active)
        VALUES
            ($1, $2, $3, $4, $5, true)
        ON CONFLICT (agency_id, provider_type, provider_key)
        DO UPDATE SET
            credentials = EXCLUDED.credentials,
            settings    = EXCLUDED.settings,
            is_active   = true,
            updated_at  = now()
        "#,
        user.agency_id,
        provider_type,
        provider_key,
        encrypted,
        body.settings,
    )
    .execute(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    // Reload provider in registry so it takes effect immediately.
    state
        .custom()
        .providers
        .load_agency(user.agency_id, state.infra.tenant_pools.platform(), enc_key)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    // Audit
    state
        .custom()
        .audit
        .log(crate::infrastructure::audit::AuditEvent {
            agency_id: user.agency_id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: format!("integration.{provider_type}.{provider_key}.upserted"),
            entity_type: "agency_integration".into(),
            entity_id: user.agency_id,
            old_data: None,
            new_data: Some(serde_json::json!({
                "provider_type": provider_type,
                "provider_key":  provider_key,
            })),
            ip_address: None,
        });

    Ok(StatusCode::OK)
}

// ── DELETE /api/agency/integrations/:type/:key ────────────────────────────────

pub async fn delete_integration(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((provider_type, provider_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    sqlx::query!(
        r#"
        UPDATE agency_integrations
        SET    is_active  = false,
               updated_at = now()
        WHERE  agency_id     = $1
          AND  provider_type = $2
          AND  provider_key  = $3
        "#,
        user.agency_id,
        provider_type,
        provider_key,
    )
    .execute(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    // Remove from live registry.
    match provider_type.as_str() {
        "sms" => {
            state.custom().providers.sms.agency.remove(&user.agency_id);
        }
        "email" => {
            state.custom().providers.email.remove(&user.agency_id);
        }
        "payment" => {
            state.custom().providers.payment.remove(&user.agency_id);
        }
        "storage" => {
            state.custom().providers.storage.remove(&user.agency_id);
        }
        _ => {}
    }

    state
        .custom()
        .audit
        .log(crate::infrastructure::audit::AuditEvent {
            agency_id: user.agency_id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: format!("integration.{provider_type}.{provider_key}.deactivated"),
            entity_type: "agency_integration".into(),
            entity_id: user.agency_id,
            old_data: None,
            new_data: None,
            ip_address: None,
        });

    Ok(StatusCode::NO_CONTENT)
}

use sqlx;
