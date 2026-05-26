// src/presentation/http/handlers/agency_integrations.rs
//
//   GET    /api/agency/integrations
//   PUT    /api/agency/integrations/:type/:key
//   DELETE /api/agency/integrations/:type/:key
//
// All handlers require "agency_settings:write".
//
// Handler responsibilities (only):
//   1. Permission check
//   2. Call the use case
//   3. Reload / evict the live provider registry  ← runtime infra, not business logic
//   4. Emit audit event

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
    application::{
        errors::AppError,
        use_cases::agency_integration::{
            deactivate_integration::{DeactivateIntegrationInput, DeactivateIntegrationUseCase},
            list_integrations::ListIntegrationsUseCase,
            upsert_integration::{UpsertIntegrationInput, UpsertIntegrationUseCase},
        },
    },
    domain::{agency_integration::AgencyIntegration, auth::AuthenticatedUser},
    infrastructure::db::{
        agency_integration_repository_sqlx::PgAgencyIntegrationRepo, pool::AgencyPool,
    },
    presentation::app_state::AppState,
};

// ── Response DTO ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct IntegrationResponse {
    pub provider_type: String,
    pub provider_key: String,
    pub is_active: bool,
    pub settings: serde_json::Value,
    /// Credentials are write-only and are never returned.
    pub credentials: &'static str,
}

impl From<AgencyIntegration> for IntegrationResponse {
    fn from(i: AgencyIntegration) -> Self {
        Self {
            provider_type: i.provider_type,
            provider_key: i.provider_key,
            is_active: i.is_active,
            settings: i.settings,
            credentials: "[redacted]",
        }
    }
}

// ── Request DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertIntegrationRequest {
    /// Plain-text credentials — shape depends on provider:
    ///   mpesa:           { consumer_key, consumer_secret, passkey }
    ///   africas_talking: { api_key, username, sender_id? }
    ///   sendgrid:        { api_key, from_email?, from_name? }
    ///   twilio:          { account_sid, auth_token, from_number }
    pub credentials: serde_json::Value,
    #[serde(default)]
    pub settings: serde_json::Value,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_integrations(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(AgencyPool(pool)): Extension<AgencyPool>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgAgencyIntegrationRepo::new(
        pool,
        state.customisation().enc_key.clone(),
    ));

    let integrations = ListIntegrationsUseCase::new(repo)
        .execute(user.agency_id)
        .await?;

    Ok(Json(
        integrations
            .into_iter()
            .map(IntegrationResponse::from)
            .collect::<Vec<_>>(),
    ))
}

pub async fn upsert_integration(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(AgencyPool(pool)): Extension<AgencyPool>,
    Path((provider_type, provider_key)): Path<(String, String)>,
    Json(body): Json<UpsertIntegrationRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgAgencyIntegrationRepo::new(
        pool.clone(),
        state.customisation().enc_key.clone(),
    ));

    UpsertIntegrationUseCase::new(repo)
        .execute(UpsertIntegrationInput {
            agency_id: user.agency_id,
            provider_type: provider_type.clone(),
            provider_key: provider_key.clone(),
            credentials: body.credentials,
            settings: body.settings,
        })
        .await?;

    // Reload the live provider registry so the new credentials take effect
    // immediately. This is a runtime infrastructure side effect — it belongs
    // here, not in the use case.
    let enc_key = &*state.customisation().enc_key;
    state
        .customisation()
        .providers
        .load_agency(user.agency_id, &pool, enc_key)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    state
        .customisation()
        .audit
        .log(crate::infrastructure::audit::AuditEvent {
            agency_id: user.agency_id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: format!("integration.{provider_type}.{provider_key}.upserted"),
            entity_type: "agency_integration".into(),
            entity_id: user.agency_id,
            old_data: None,
            new_data: Some(
                serde_json::json!({ "provider_type": provider_type, "provider_key": provider_key }),
            ),
            ip_address: None,
        });

    Ok(StatusCode::OK)
}

pub async fn delete_integration(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(AgencyPool(pool)): Extension<AgencyPool>,
    Path((provider_type, provider_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgAgencyIntegrationRepo::new(
        pool,
        state.customisation().enc_key.clone(),
    ));

    DeactivateIntegrationUseCase::new(repo)
        .execute(DeactivateIntegrationInput {
            agency_id: user.agency_id,
            provider_type: provider_type.clone(),
            provider_key: provider_key.clone(),
        })
        .await?;

    // Evict the now-inactive provider from the live in-memory registry.
    match provider_type.as_str() {
        "sms" => {
            state.customisation().providers.sms.agency.remove(&user.agency_id);
        }
        "email" => {
            state.customisation().providers.email.remove(&user.agency_id);
        }
        "payment" => {
            state.customisation().providers.payment.remove(&user.agency_id);
        }
        "storage" => {
            state.customisation().providers.storage.remove(&user.agency_id);
        }
        _ => {}
    }

    state
        .customisation()
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
