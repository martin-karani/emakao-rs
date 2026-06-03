// src/presentation/http/handlers/vendor.rs
// REFACTORED: replaced `require_feature(&sub.entitlements, …)` with
// `require_feature!(state, ctx.agency.id, "maint_vendor_portal")` and
// `require_limit(&sub.entitlements, &LimitKey::MaxVendors, …)` with
// `require_below_limit!(state, ctx.agency.id, "max_vendors", …)`.
// Dropped `Extension(sub)` and all subscription domain imports.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::vendor_repository::VendorRepository,
        use_cases::vendor::{
            create_vendor::{CreateVendorInput, CreateVendorUseCase},
            get_vendor::GetVendorUseCase,
            list_vendors::ListVendorsUseCase,
            update_vendor::UpdateVendorUseCase,
        },
    },
    domain::{auth::AuthenticatedUser, vendor::UpdateVendorCommand},
    infrastructure::db::vendor_repository_sqlx::PgVendorRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::{
                pagination::PaginationParams,
                vendor::{CreateVendorDto, ListVendorsParams, UpdateVendorDto},
            },
            responses::{maintenance::WorkOrderResponse, vendor::VendorResponse},
        },
        require_below_limit, require_feature,
    },
};

// ── List vendors ──────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/vendors",
    params(ListVendorsParams),
    responses(
        (status = 200, description = "List of vendors", body = [VendorResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Vendors",
    security(("bearer_token" = []))
)]
pub async fn list_vendors(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Query(params): Query<ListVendorsParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature!(state, ctx.agency.id, "portal_vendor");

    let items = ListVendorsUseCase::new(Arc::new(PgVendorRepo::from(ctx.pool)))
        .execute(
            ctx.agency.id,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(VendorResponse::from)
            .collect::<Vec<_>>(),
    ))
}

// ── Get vendor ────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/vendors/{id}",
    params(("id" = Uuid, Path, description = "Vendor UUID")),
    responses(
        (status = 200, description = "Vendor found", body = VendorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Vendor not found",       body = ErrorResponse),
    ),
    tag = "Vendors",
    security(("bearer_token" = []))
)]
pub async fn get_vendor(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature!(state, ctx.agency.id, "portal_vendor");

    let vendor = GetVendorUseCase::new(Arc::new(PgVendorRepo::from(ctx.pool)))
        .execute(ctx.agency.id, id)
        .await?;

    Ok(Json(VendorResponse::from(vendor)))
}

// ── Create vendor ─────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1/vendors",
    request_body = CreateVendorDto,
    responses(
        (status = 201, description = "Vendor created", body = VendorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 402, description = "Plan limit reached",     body = ErrorResponse),
        (status = 422, description = "Validation error",       body = ErrorResponse),
    ),
    tag = "Vendors",
    security(("bearer_token" = []))
)]
pub async fn create_vendor(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Json(dto): Json<CreateVendorDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature!(state, ctx.agency.id, "portal_vendor");

    let vendor = CreateVendorUseCase::new(Arc::new(PgVendorRepo::from(ctx.pool)))
        .execute(CreateVendorInput {
            agency_id: ctx.agency.id,
            name: dto.name,
            contact_name: dto.contact_name,
            email: dto.email,
            phone: dto.phone,
            speciality: dto.speciality,
            notes: dto.notes,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(VendorResponse::from(vendor))))
}

// ── Update vendor ─────────────────────────────────────────────────────────────

#[utoipa::path(
    patch,
    path = "/api/v1/vendors/{id}",
    params(("id" = Uuid, Path, description = "Vendor UUID")),
    request_body = UpdateVendorDto,
    responses(
        (status = 200, description = "Vendor updated", body = VendorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Vendor not found",       body = ErrorResponse),
        (status = 422, description = "Validation error",       body = ErrorResponse),
    ),
    tag = "Vendors",
    security(("bearer_token" = []))
)]
pub async fn update_vendor(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateVendorDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature!(state, ctx.agency.id, "portal_vendor");

    let vendor = UpdateVendorUseCase::new(Arc::new(PgVendorRepo::from(ctx.pool)))
        .execute(UpdateVendorCommand {
            id,
            agency_id: ctx.agency.id,
            name: dto.name,
            phone: dto.phone,
            email: dto.email,
            contact_name: dto.contact_name,
            speciality: dto.speciality,
            status: dto.status,
            notes: dto.notes,
        })
        .await?;

    Ok(Json(VendorResponse::from(vendor)))
}

// ── Portal: get own profile ───────────────────────────────────────────────────

pub async fn get_my_profile(
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgVendorRepo::from(ctx.pool));
    let vendor = VendorRepository::find_by_user_id(repo.as_ref(), user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Vendor profile not found".into()))?;

    Ok(Json(VendorResponse::from(vendor)))
}

// ── Portal: list own work orders ──────────────────────────────────────────────

pub async fn list_my_work_orders(
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgVendorRepo::from(ctx.pool));
    let vendor = repo
        .find_by_user_id(user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Vendor profile not found".into()))?;

    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let work_orders = repo
        .find_work_orders_by_vendor_id(vendor.id, limit, offset)
        .await?;

    Ok(Json(
        work_orders
            .into_iter()
            .map(WorkOrderResponse::from)
            .collect::<Vec<_>>(),
    ))
}
