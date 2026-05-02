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
    application::use_cases::vendor::{
        create_vendor::CreateVendorUseCase, get_vendor::GetVendorUseCase,
        list_vendors::ListVendorsUseCase, update_vendor::UpdateVendorUseCase,
    },
    application::{errors::AppError, use_cases::vendor::create_vendor::CreateVendorInput},
    domain::{subscription::FeatureKey, vendor::UpdateVendorCommand},
    infrastructure::db::vendor_repository_sqlx::PgVendorRepo,
    presentation::{
        app_state::AppState,
        extractors::TenantContext,
        http::{
            dto::vendor::{CreateVendorDto, ListVendorsParams, UpdateVendorDto},
            responses::vendor::VendorResponse,
        },
        middleware::subscription::{require_feature, ResolvedSubscription},
    },
};

pub async fn list_vendors(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListVendorsParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintVendorPortal)?;

    let repo = Arc::new(PgVendorRepo::from(ctx.pool));
    let usecase = ListVendorsUseCase::new(repo);

    let items = usecase
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

pub async fn get_vendor(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::MaintVendorPortal)?;

    let repo = Arc::new(PgVendorRepo::from(ctx.pool));
    let usecase = GetVendorUseCase::new(repo);

    let vendor = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(VendorResponse::from(vendor)))
}

pub async fn create_vendor(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateVendorDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintVendorPortal)?;

    let current = state
        .subscription
        .repo
        .count_tenant_rows(ctx.agency.id, "vendors")
        .await?;
    crate::presentation::middleware::subscription::require_limit(
        &sub.entitlements,
        &crate::domain::subscription::LimitKey::MaxVendors,
        current,
    )?;

    let repo = Arc::new(PgVendorRepo::from(ctx.pool));
    let usecase = CreateVendorUseCase::new(repo);

    let vendor = usecase
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

pub async fn update_vendor(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateVendorDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::MaintVendorPortal)?;

    let repo = Arc::new(PgVendorRepo::from(ctx.pool));
    let usecase = UpdateVendorUseCase::new(repo);

    let vendor = usecase
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
