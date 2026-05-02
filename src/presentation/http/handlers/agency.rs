use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use garde::Validate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::{errors::AppError, use_cases::agency::provision::ProvisionAgencyInput},
    presentation::app_state::AppState,
};

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Validate)]
pub struct CreateAgencyDto {
    #[garde(length(min = 2, max = 120))]
    pub name: String,

    /// Lowercase, hyphen-separated, e.g. "acme-realty".
    /// Must be unique; becomes both the URL slug and the Postgres schema prefix.
    #[garde(pattern(r"^[a-z0-9]+(?:-[a-z0-9]+)*$"), length(min = 2, max = 63))]
    pub slug: String,

    #[garde(length(min = 2, max = 2))]
    pub country_code: String,

    #[garde(length(min = 3, max = 3))]
    pub currency_code: String,
}

#[derive(Serialize)]
pub struct AgencyResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub schema_name: String,
    pub fga_store_id: Option<String>,
    pub status: String,
}

// ── Tuple management DTOs ─────────────────────────────────────────────────────

/// Body for `POST /api/v1/admin/agencies/:agency_id/permissions/tuples`.
#[derive(Deserialize, Validate)]
pub struct WriteTupleDto {
    /// e.g. "user:550e8400-e29b-41d4-a716-446655440000"
    #[garde(length(min = 5))]
    pub user: String,
    /// e.g. "manager", "viewer", "admin"
    #[garde(length(min = 1))]
    pub relation: String,
    /// e.g. "property:123e4567-e89b-12d3-a456-426614174000"
    #[garde(length(min = 5))]
    pub object: String,
}

/// Body for `DELETE /api/v1/admin/agencies/:agency_id/permissions/tuples`.
#[derive(Deserialize)]
pub struct DeleteTupleDto {
    pub user: String,
    pub relation: String,
    pub object: String,
}

/// Body for `POST /api/v1/admin/agencies/:agency_id/permissions/model`.
///
/// Pass a full OpenFGA JSON model.  A new immutable model version is created
/// in the agency's store; existing tuples continue to resolve correctly until
/// the next `check` call picks up the new version.
#[derive(Deserialize)]
pub struct UpdateAuthModelDto {
    /// Full authorization model JSON, e.g.:
    /// `{ "schema_version": "1.1", "type_definitions": [...] }`
    pub model: serde_json::Value,
}

#[derive(Serialize)]
pub struct ModelVersionResponse {
    pub authorization_model_id: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/v1/admin/agencies
///
/// Creates an agency row, provisions the tenant Postgres schema (migrations
/// included), creates an OpenFGA store, seeds the default authorization model,
/// and persists the `store_id`.  All in one atomic-ish flow.
pub async fn create_agency(
    State(state): State<AppState>,
    Json(dto): Json<CreateAgencyDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let agency = state
        .agency
        .provision
        .execute(ProvisionAgencyInput {
            name: dto.name,
            slug: dto.slug,
            country_code: dto.country_code,
            currency_code: dto.currency_code,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(AgencyResponse {
            id: agency.id,
            name: agency.name,
            slug: agency.slug,
            schema_name: agency.schema_name,
            fga_store_id: agency.fga_store_id,
            status: agency.status,
        }),
    ))
}

/// POST /api/v1/admin/agencies/:fga_store_id/permissions/tuples
///
/// Grant a relation tuple inside the agency's FGA store.
///
/// Example body:
/// ```json
/// { "user": "user:abc123", "relation": "manager", "object": "property:xyz789" }
/// ```
pub async fn write_permission_tuple(
    State(state): State<AppState>,
    axum::extract::Path(fga_store_id): axum::extract::Path<String>,
    Json(dto): Json<WriteTupleDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    state
        .openfga
        .write_tuple(&fga_store_id, &dto.user, &dto.relation, &dto.object)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/admin/agencies/:fga_store_id/permissions/tuples
///
/// Revoke a relation tuple from the agency's FGA store.
pub async fn delete_permission_tuple(
    State(state): State<AppState>,
    axum::extract::Path(fga_store_id): axum::extract::Path<String>,
    Json(dto): Json<DeleteTupleDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .openfga
        .delete_tuple(&fga_store_id, &dto.user, &dto.relation, &dto.object)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/admin/agencies/:fga_store_id/permissions/model
///
/// Publish a new authorization model version into the agency's FGA store.
///
/// Use this when an agency needs custom types or relations beyond the default
/// model.  Old model versions remain intact; only new `check` calls will
/// evaluate the new version.
pub async fn update_auth_model(
    State(state): State<AppState>,
    axum::extract::Path(fga_store_id): axum::extract::Path<String>,
    Json(dto): Json<UpdateAuthModelDto>,
) -> Result<impl IntoResponse, AppError> {
    let model_id = state
        .openfga
        .write_auth_model(&fga_store_id, &dto.model)
        .await?;

    Ok(Json(ModelVersionResponse {
        authorization_model_id: model_id,
    }))
}
