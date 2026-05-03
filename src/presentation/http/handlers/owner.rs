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
        ports::owner_repository::OwnerRepository,
        use_cases::owner::{
            create_owner::{CreateOwnerInput, CreateOwnerUseCase},
            get_owner::GetOwnerUseCase,
            list_owners::ListOwnersUseCase,
            onboard_owner::{OnboardOwnerInput, OnboardOwnerUseCase},
            update_owner::UpdateOwnerUseCase,
        },
    },
    domain::{
        auth::AuthenticatedUser,
        owner::UpdateOwnerCommand,
        subscription::{FeatureKey, LimitKey},
    },
    infrastructure::db::owner_repository_sqlx::PgOwnerRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::{
                owner::{AssignOwnerDto, CreateOwnerDto, ListOwnersParams, UpdateOwnerDto},
                pagination::PaginationParams,
            },
            responses::{
                owner::{DisbursementResponse, OwnerResponse},
                property::PropertyWithPercentResponse,
            },
        },
        middleware::subscription::{require_feature, require_limit, ResolvedSubscription},
    },
};

/// List property owners (staff)
#[utoipa::path(
    get,
    path = "/api/v1/owners",
    params(ListOwnersParams),
    responses(
        (status = 200, description = "List of owners", body = Vec<OwnerResponse>),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn list_owners(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListOwnersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let usecase = ListOwnersUseCase::new(repo);

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
            .map(OwnerResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Get owner details (staff)
#[utoipa::path(
    get,
    path = "/api/v1/owners/{id}",
    params(("id" = Uuid, Path, description = "Owner UUID")),
    responses(
        (status = 200, description = "Owner details", body = OwnerResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Owner not found", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn get_owner(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let usecase = GetOwnerUseCase::new(repo);

    let owner = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(OwnerResponse::from(owner)))
}

/// Create a new property owner (staff)
#[utoipa::path(
    post,
    path = "/api/v1/owners",
    request_body = CreateOwnerDto,
    responses(
        (status = 201, description = "Owner created", body = OwnerResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn create_owner(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateOwnerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let current = state
        .subscription
        .repo
        .count_tenant_rows(ctx.agency.id, "owners")
        .await?;
    require_limit(&sub.entitlements, &LimitKey::MaxOwners, current)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let usecase = CreateOwnerUseCase::new(repo);
    let owner = usecase
        .execute(CreateOwnerInput {
            agency_id: ctx.agency.id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            email: Some(dto.email),
            phone: dto.phone,
            company_name: dto.company_name,
            kra_pin: dto.kra_pin,
            bank_name: dto.bank_name,
            bank_account: dto.bank_account,
            mpesa_number: dto.mpesa_number,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(OwnerResponse::from(owner))))
}

/// Invite/Onboard a new property owner (staff)
#[utoipa::path(
    post,
    path = "/api/v1/owners/invite",
    request_body = CreateOwnerDto,
    responses(
        (status = 201, description = "Owner invited", body = OwnerResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn invite_owner(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateOwnerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let current = state
        .subscription
        .repo
        .count_tenant_rows(ctx.agency.id, "owners")
        .await?;
    require_limit(&sub.entitlements, &LimitKey::MaxOwners, current)?;

    let uc = OnboardOwnerUseCase {
        owner_repo: Arc::new(PgOwnerRepo::from(ctx.pool)),
        auth_repo: state.auth_repo.clone(),
        auth_port: state.auth_port.clone(),
        email: state.email.clone(),
        sms: state.sms.clone(),
    };

    let owner = uc
        .execute(OnboardOwnerInput {
            agency_id: ctx.agency.id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            email: Some(dto.email),
            phone: dto.phone,
            company_name: dto.company_name,
            kra_pin: dto.kra_pin,
            bank_name: dto.bank_name,
            bank_account: dto.bank_account,
            mpesa_number: dto.mpesa_number,
            portal_base_url: "https://owners.emakao.co.ke".to_string(), // TODO: from config
        })
        .await?;

    Ok((StatusCode::CREATED, Json(OwnerResponse::from(owner))))
}
/// Update owner details (staff)
#[utoipa::path(
    patch,
    path = "/api/v1/owners/{id}",
    params(("id" = Uuid, Path, description = "Owner UUID")),
    request_body = UpdateOwnerDto,
    responses(
        (status = 200, description = "Owner updated", body = OwnerResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Owner not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn update_owner(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateOwnerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let usecase = UpdateOwnerUseCase::new(repo);

    let owner = usecase
        .execute(UpdateOwnerCommand {
            id,
            agency_id: ctx.agency.id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            phone: dto.phone,
            company_name: dto.company_name,
            kra_pin: dto.kra_pin,
            bank_name: dto.bank_name,
            bank_account: dto.bank_account,
            mpesa_number: dto.mpesa_number,
        })
        .await?;

    Ok(Json(OwnerResponse::from(owner)))
}

/// Assign an owner to a property (staff)
#[utoipa::path(
    post,
    path = "/api/v1/properties/{propertyId}/owners",
    params(("propertyId" = Uuid, Path, description = "Property UUID")),
    request_body = AssignOwnerDto,
    responses(
        (status = 204, description = "Owner assigned"),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Property or owner not found", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn assign_owner_to_property(
    ctx: AgencyContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(property_id): Path<Uuid>,
    Json(dto): Json<AssignOwnerDto>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::OwnerPortal)?;

    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    repo.assign_to_property(dto.owner_id, property_id, dto.ownership_percent)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get own profile (using JWT user_id)
#[utoipa::path(
    get,
    path = "/api/v1/owners/me",
    responses(
        (status = 200, description = "Current owner profile", body = OwnerResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Owner profile not found", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn get_my_profile(
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let owner = repo
        .find_by_user_id(user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Owner profile not found".into()))?;
    Ok(Json(OwnerResponse::from(owner)))
}

#[utoipa::path(
    get,
    path = "/api/v1/owners/me/properties",
    params(PaginationParams),
    responses(
        (status = 200, description = "Current owner's properties", body = [PropertyWithPercentResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Owner profile not found", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn list_my_properties(
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    // First, get the owner internal id from user_id
    let owner = repo
        .find_by_user_id(user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Owner profile not found".into()))?;
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let properties = repo
        .find_properties_by_owner_id(owner.id, limit, offset)
        .await?;
    let responses: Vec<PropertyWithPercentResponse> = properties
        .into_iter()
        .map(|(prop, percent)| {
            crate::presentation::http::responses::property::PropertyWithPercentResponse::from(
                prop, percent,
            )
        })
        .collect();
    Ok(Json(responses))
}

#[utoipa::path(
    get,
    path = "/api/v1/owners/me/disbursements",
    params(PaginationParams),
    responses(
        (status = 200, description = "Current owner's disbursements", body = [DisbursementResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Owner profile not found", body = ErrorResponse),
    ),
    tag = "Owners",
    security(("bearer_token" = []))
)]
pub async fn list_my_disbursements(
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgOwnerRepo::from(ctx.pool));
    let owner = repo
        .find_by_user_id(user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Owner profile not found".into()))?;
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let disbursements = repo
        .find_disbursements_by_owner_id(owner.id, limit, offset)
        .await?;
    let responses: Vec<DisbursementResponse> = disbursements.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}
