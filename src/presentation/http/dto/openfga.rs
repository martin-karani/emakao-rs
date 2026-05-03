use serde::Deserialize;
use utoipa::ToSchema;

use garde::Validate;

/// Body for `POST /api/v1/admin/agencies/:agency_id/permissions/tuples`.
#[derive(Deserialize, Validate, ToSchema)]
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
#[derive(Deserialize, ToSchema)]
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
#[derive(Deserialize, ToSchema)]
pub struct UpdateAuthModelDto {
    /// Full authorization model JSON, e.g.:
    /// `{ "schema_version": "1.1", "type_definitions": [...] }`
    #[schema(value_type = Object)]
    pub model: serde_json::Value,
}
