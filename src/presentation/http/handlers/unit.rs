// src/presentation/http/handlers/unit.rs
//
// HTTP handlers for unit CRUD:
//   POST   /api/v1/properties/{propertyId}/units   — create one or more units
//   GET    /api/v1/properties/{propertyId}/units   — list units for a property
//   GET    /api/v1/units/{unitId}                  — get a single unit
//   PUT    /api/v1/units/{unitId}                  — update a unit
//   DELETE /api/v1/units/{unitId}                  — delete a unit (no active lease)
//
// Permission model:
//   • Creating / editing / deleting a unit requires `can_edit` on `property:{id}`.
//   • Reading a unit requires `can_view` on `property:{id}`.
//   • The agency-id is always verified by the middleware (AgencyContext), so we
//     only need to resolve the property_id from the path / unit record.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{property_repository::PropertyRepository, unit_repository::UnitRepository},
    },
    domain::{
        auth::AuthenticatedUser,
        property::{CreateUnitCommand, UpdateUnitCommand},
    },
    infrastructure::db::{
        property_repository_sqlx::PgPropertyRepo, unit_repository_sqlx::PgUnitRepo,
    },
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::unit::{CreateUnitsDto, UpdateUnitDto},
            helpers::permission::check_permission,
            responses::unit::UnitResponse,
        },
    },
};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve the property_id that owns a unit, verify it belongs to the agency,
/// and run the given OpenFGA permission check on `property:{property_id}`.
async fn resolve_and_check(
    state: &AppState,
    ctx: &AgencyContext,
    user: &AuthenticatedUser,
    unit_repo: &PgUnitRepo,
    prop_repo: &PgPropertyRepo,
    unit_id: Uuid,
    relation: &str,
) -> Result<Uuid, AppError> {
    let property_id = unit_repo
        .property_id_for_unit(unit_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("unit {unit_id}")))?;

    // Verify the property belongs to the authenticated agency.
    prop_repo
        .find_by_id(ctx.agency.id, property_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("property {property_id}")))?;

    check_permission(
        state,
        ctx,
        user,
        relation,
        &format!("property:{property_id}"),
    )
    .await?;

    Ok(property_id)
}

// ── List units ────────────────────────────────────────────────────────────────

/// List all units for a property.
#[utoipa::path(
    get,
    path = "/api/v1/properties/{propertyId}/units",
    params(("propertyId" = Uuid, Path, description = "Property UUID")),
    responses(
        (status = 200, description = "List of units",           body = Vec<UnitResponse>),
        (status = 401, description = "Missing or invalid JWT",  body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Property not found",      body = ErrorResponse),
    ),
    tag = "Units",
    security(("bearer_token" = []))
)]
pub async fn list_units(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(property_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // Verify property belongs to this agency.
    let prop_repo = Arc::new(PgPropertyRepo::from(ctx.pool.clone()));
    prop_repo
        .find_by_id(ctx.agency.id, property_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("property {property_id}")))?;

    check_permission(
        &state,
        &ctx,
        &user,
        "can_view",
        &format!("property:{property_id}"),
    )
    .await?;

    let unit_repo = Arc::new(PgUnitRepo::from(ctx.pool));
    let units = unit_repo.find_all_for_property(property_id).await?;

    Ok(Json(
        units
            .into_iter()
            .map(UnitResponse::from)
            .collect::<Vec<_>>(),
    ))
}

// ── Get single unit ───────────────────────────────────────────────────────────

/// Get a single unit by UUID.
#[utoipa::path(
    get,
    path = "/api/v1/units/{unitId}",
    params(("unitId" = Uuid, Path, description = "Unit UUID")),
    responses(
        (status = 200, description = "Unit found",              body = UnitResponse),
        (status = 401, description = "Missing or invalid JWT",  body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Unit not found",          body = ErrorResponse),
    ),
    tag = "Units",
    security(("bearer_token" = []))
)]
pub async fn get_unit(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(unit_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let unit_repo = Arc::new(PgUnitRepo::from(ctx.pool.clone()));
    let prop_repo = Arc::new(PgPropertyRepo::from(ctx.pool.clone()));

    resolve_and_check(
        &state, &ctx, &user, &unit_repo, &prop_repo, unit_id, "can_view",
    )
    .await?;

    let unit = unit_repo
        .find_by_id(unit_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("unit {unit_id}")))?;

    Ok(Json(UnitResponse::from(unit)))
}

// ── Create units (batch) ──────────────────────────────────────────────────────

/// Create one or more units for a property in a single transaction.
///
/// Sending multiple units at once is preferred over sequential single-unit
/// calls because the entire batch is committed atomically — no partial states.
#[utoipa::path(
    post,
    path = "/api/v1/properties/{propertyId}/units",
    params(("propertyId" = Uuid, Path, description = "Property UUID")),
    request_body = CreateUnitsDto,
    responses(
        (status = 201, description = "Units created",           body = Vec<UnitResponse>),
        (status = 401, description = "Missing or invalid JWT",  body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Property not found",      body = ErrorResponse),
        (status = 409, description = "Duplicate unit_number",   body = ErrorResponse),
        (status = 422, description = "Validation error",        body = ErrorResponse),
    ),
    tag = "Units",
    security(("bearer_token" = []))
)]
pub async fn create_units(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(property_id): Path<Uuid>,
    Json(dto): Json<CreateUnitsDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    // Validate unit_numbers are unique within the batch (DB will also catch
    // conflicts with existing units, but this gives a cleaner error message).
    let mut seen = std::collections::HashSet::new();
    for u in &dto.units {
        if !seen.insert(u.unit_number.trim().to_lowercase()) {
            return Err(AppError::Conflict(format!(
                "duplicate unit_number '{}' in request body",
                u.unit_number
            )));
        }
    }

    // Verify property belongs to this agency.
    let prop_repo = Arc::new(PgPropertyRepo::from(ctx.pool.clone()));
    prop_repo
        .find_by_id(ctx.agency.id, property_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("property {property_id}")))?;

    check_permission(
        &state,
        &ctx,
        &user,
        "can_edit",
        &format!("property:{property_id}"),
    )
    .await?;

    let unit_repo = Arc::new(PgUnitRepo::from(ctx.pool));
    let cmds: Vec<CreateUnitCommand> = dto
        .units
        .into_iter()
        .map(|u| CreateUnitCommand {
            property_id,
            unit_type_id: u.unit_type_id,
            unit_number: u.unit_number.trim().to_owned(),
            floor: u.floor,
            size_sqm: u.size_sqm,
            bedrooms: u.bedrooms,
            bathrooms: u.bathrooms,
            rent_amount_kes: u.rent_amount_kes,
            deposit_kes: u.deposit_kes.unwrap_or_default(),
            description: u.description,
        })
        .collect();

    let units = unit_repo.create_batch(cmds).await?;

    // ── OpenFGA Tuples ───────────────────────────────────────────────────
    if let Some(ref store_id) = ctx.agency.fga_store_id {
        let property_obj = format!("property:{property_id}");

        for unit in &units {
            let unit_obj = format!("unit:{}", unit.id);
            // Link unit to property
            let _ = state
                .openfga
                .write_tuple(store_id, &property_obj, "parent_property", &unit_obj)
                .await;
        }

        tracing::info!(
            property_id = %property_id,
            count = units.len(),
            "OpenFGA parent_property tuples written for batch"
        );
    }

    Ok((
        StatusCode::CREATED,
        Json(
            units
                .into_iter()
                .map(UnitResponse::from)
                .collect::<Vec<_>>(),
        ),
    ))
}

// ── Update unit ───────────────────────────────────────────────────────────────

/// Update mutable fields of a unit.
#[utoipa::path(
    put,
    path = "/api/v1/units/{unitId}",
    params(("unitId" = Uuid, Path, description = "Unit UUID")),
    request_body = UpdateUnitDto,
    responses(
        (status = 200, description = "Unit updated",            body = UnitResponse),
        (status = 401, description = "Missing or invalid JWT",  body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Unit not found",          body = ErrorResponse),
        (status = 409, description = "Duplicate unit_number",   body = ErrorResponse),
        (status = 422, description = "Validation error",        body = ErrorResponse),
    ),
    tag = "Units",
    security(("bearer_token" = []))
)]
pub async fn update_unit(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(unit_id): Path<Uuid>,
    Json(dto): Json<UpdateUnitDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let unit_repo = Arc::new(PgUnitRepo::from(ctx.pool.clone()));
    let prop_repo = Arc::new(PgPropertyRepo::from(ctx.pool.clone()));

    let property_id = resolve_and_check(
        &state, &ctx, &user, &unit_repo, &prop_repo, unit_id, "can_edit",
    )
    .await?;

    let updated = unit_repo
        .update(UpdateUnitCommand {
            id: unit_id,
            property_id,
            unit_type_id: dto.unit_type_id,
            unit_number: dto.unit_number,
            // For nullable scalar columns we use Option<Option<T>>:
            // Some(Some(v)) → set to v; Some(None) → set to NULL; None → unchanged.
            // The DTO uses Option<T> (no explicit null), so we wrap one level.
            floor: dto.floor.map(Some),
            size_sqm: dto.size_sqm.map(Some),
            bedrooms: dto.bedrooms.map(Some),
            bathrooms: dto.bathrooms.map(Some),
            rent_amount_kes: dto.rent_amount_kes,
            deposit_kes: dto.deposit_kes,
            status: dto.status,
            description: dto.description.map(Some),
        })
        .await?;

    Ok(Json(UnitResponse::from(updated)))
}

// ── Delete unit ───────────────────────────────────────────────────────────────

/// Delete a unit.
///
/// Returns **409 Conflict** when the unit has an active lease agreement,
/// preventing accidental data loss.
#[utoipa::path(
    delete,
    path = "/api/v1/units/{unitId}",
    params(("unitId" = Uuid, Path, description = "Unit UUID")),
    responses(
        (status = 204, description = "Unit deleted"),
        (status = 401, description = "Missing or invalid JWT",  body = ErrorResponse),
        (status = 403, description = "Insufficient permission", body = ErrorResponse),
        (status = 404, description = "Unit not found",          body = ErrorResponse),
        (status = 409, description = "Unit has an active lease",body = ErrorResponse),
    ),
    tag = "Units",
    security(("bearer_token" = []))
)]
pub async fn delete_unit(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(unit_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let unit_repo = Arc::new(PgUnitRepo::from(ctx.pool.clone()));
    let prop_repo = Arc::new(PgPropertyRepo::from(ctx.pool.clone()));

    resolve_and_check(
        &state, &ctx, &user, &unit_repo, &prop_repo, unit_id, "can_edit",
    )
    .await?;

    // Guard: refuse deletion if an active agreement exists.
    if unit_repo.has_active_agreement(unit_id).await? {
        return Err(AppError::Conflict(
            "Cannot delete a unit that has an active lease agreement. \
             Terminate the agreement first."
                .into(),
        ));
    }

    unit_repo.delete(unit_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
