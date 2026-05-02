use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::use_cases::utility::{
        create_meter::CreateMeterUseCase, generate_bill::GenerateBillUseCase,
        record_reading::RecordReadingUseCase,
    },
    application::{
        errors::AppError,
        use_cases::utility::{create_meter::CreateMeterInput, record_reading::RecordReadingInput},
    },
    domain::{auth::AuthenticatedUser, subscription::FeatureKey},
    infrastructure::db::utility_repository_sqlx::PgUtilityRepo,
    presentation::{
        app_state::AppState,
        extractors::TenantContext,
        http::{
            dto::utility::{CreateMeterDto, RecordReadingDto},
            responses::utility::UtilityMeterResponse,
        },
        middleware::subscription::{require_feature, ResolvedSubscription},
    },
};

pub async fn create_meter(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateMeterDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = CreateMeterUseCase::new(repo);

    let meter = usecase
        .execute(CreateMeterInput {
            unit_id: dto.unit_id,
            meter_type: dto.meter_type,
            billing_mode: dto.billing_mode,
            meter_number: dto.meter_number,
            rate_per_unit: dto.rate_per_unit,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(UtilityMeterResponse::from(meter))))
}

pub async fn record_reading(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Json(dto): Json<RecordReadingDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = RecordReadingUseCase::new(repo);

    usecase
        .execute(RecordReadingInput {
            meter_id: id,
            reading_value: dto.reading_value,
            recorded_by: user.user_id,
        })
        .await?;

    Ok(StatusCode::CREATED)
}

pub async fn generate_bill(
    State(state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = GenerateBillUseCase::new(repo);

    let bill = usecase.execute(id).await?;
    Ok(Json(bill))
}
