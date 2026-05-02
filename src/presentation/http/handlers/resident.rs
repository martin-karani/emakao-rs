use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use garde::Validate;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::use_cases::resident::{
        get_resident::GetResidentUseCase, invite_resident::InviteResidentUseCase,
        list_residents::ListResidentsUseCase,
    },
    application::{errors::AppError, use_cases::resident::invite_resident::InviteResidentInput},
    infrastructure::db::resident_repository_sqlx::PgResidentRepo,
    presentation::{
        app_state::AppState,
        extractors::TenantContext,
        http::{
            dto::resident::{InviteResidentDto, ListResidentsParams},
            responses::resident::ResidentResponse,
        },
    },
};

/// GET /api/v1/residents
pub async fn list_residents(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(params): Query<ListResidentsParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgResidentRepo::from(ctx.pool));
    let usecase = ListResidentsUseCase::new(repo);
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
            .map(ResidentResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /api/v1/residents/:id
pub async fn get_resident(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgResidentRepo::from(ctx.pool));
    let usecase = GetResidentUseCase::new(repo);
    let resident = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(ResidentResponse::from(resident)))
}

/// POST /api/v1/residents
pub async fn invite_resident(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(dto): Json<InviteResidentDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgResidentRepo::from(ctx.pool));
    let usecase = InviteResidentUseCase::new(repo, state.email.clone(), state.sms.clone());

    let resident = usecase
        .execute(InviteResidentInput {
            agency_id: ctx.agency.id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            email: dto.email,
            phone: dto.phone,
            national_id: dto.national_id,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(ResidentResponse::from(resident))))
}
