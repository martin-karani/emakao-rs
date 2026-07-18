use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::feedback_repository::FeedbackRepository,
        use_cases::feedback::{
            create_feedback::{CreateFeedbackInput, CreateFeedbackUseCase},
            create_feedback_reply::{CreateFeedbackReplyInput, CreateFeedbackReplyUseCase},
            get_feedback::{GetFeedbackInput, GetFeedbackUseCase},
            list_feedback::{ListFeedbackInput, ListFeedbackUseCase},
            list_feedback_replies::{ListFeedbackRepliesInput, ListFeedbackRepliesUseCase},
            update_feedback_status::{UpdateFeedbackStatusInput, UpdateFeedbackStatusUseCase},
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::feedback_repository_sqlx::PgFeedbackRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::feedback::{CreateFeedbackDto, CreateFeedbackReplyDto, UpdateFeedbackStatusDto},
            responses::feedback::{FeedbackReplyResponse, FeedbackResponse},
        },
    },
};

#[utoipa::path(
    post,
    path = "/api/v1/feedback",
    request_body = CreateFeedbackDto,
    responses(
        (status = 201, description = "Feedback created successfully", body = FeedbackResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Feedback",
    security(("bearer_token" = []))
)]
pub async fn create_feedback(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateFeedbackDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo: Arc<dyn FeedbackRepository> = Arc::new(PgFeedbackRepo::from(ctx.pool.clone()));
    let usecase = CreateFeedbackUseCase::new(repo);

    let feedback = usecase
        .execute(CreateFeedbackInput {
            agency_id: ctx.agency.id,
            user_id: user.user_id,
            feedback_type: dto.feedback_type,
            satisfaction: dto.satisfaction,
            subject: dto.subject,
            description: dto.description,
            email: dto.email,
        })
        .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(FeedbackResponse::from(feedback)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/feedback/{feedback_id}",
    params(
        ("feedback_id" = Uuid, Path, description = "Feedback ID"),
    ),
    responses(
        (status = 200, description = "Feedback retrieved successfully", body = FeedbackResponse),
        (status = 404, description = "Feedback not found", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Feedback",
    security(("bearer_token" = []))
)]
pub async fn get_feedback(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Path(feedback_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo: Arc<dyn FeedbackRepository> = Arc::new(PgFeedbackRepo::from(ctx.pool.clone()));
    let usecase = GetFeedbackUseCase::new(repo);

    let feedback = usecase
        .execute(GetFeedbackInput {
            agency_id: ctx.agency.id,
            feedback_id,
        })
        .await?;

    Ok(Json(FeedbackResponse::from(feedback)))
}

#[utoipa::path(
    get,
    path = "/api/v1/feedback",
    params(
        ("limit" = Option<i64>, Query, description = "Limit number of feedback items"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "List of feedback retrieved successfully", body = [FeedbackResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Feedback",
    security(("bearer_token" = []))
)]
pub async fn list_feedback(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(20);
    let offset = params
        .get("offset")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let repo: Arc<dyn FeedbackRepository> = Arc::new(PgFeedbackRepo::from(ctx.pool.clone()));
    let usecase = ListFeedbackUseCase::new(repo);

    let feedback_list = usecase
        .execute(ListFeedbackInput {
            agency_id: ctx.agency.id,
            limit,
            offset,
        })
        .await?;

    let response_list: Vec<FeedbackResponse> =
        feedback_list.into_iter().map(|f| f.into()).collect();
    Ok(Json(response_list))
}

#[utoipa::path(
    put,
    path = "/api/v1/feedback/{feedback_id}/status",
    params(
        ("feedback_id" = Uuid, Path, description = "Feedback ID"),
    ),
    request_body = UpdateFeedbackStatusDto,
    responses(
        (status = 200, description = "Feedback status updated successfully", body = FeedbackResponse),
        (status = 404, description = "Feedback not found", body = ErrorResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Feedback",
    security(("bearer_token" = []))
)]
pub async fn update_feedback_status(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Path(feedback_id): Path<Uuid>,
    Json(dto): Json<UpdateFeedbackStatusDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo: Arc<dyn FeedbackRepository> = Arc::new(PgFeedbackRepo::from(ctx.pool.clone()));
    let usecase = UpdateFeedbackStatusUseCase::new(repo);

    let feedback = usecase
        .execute(UpdateFeedbackStatusInput {
            agency_id: ctx.agency.id,
            feedback_id,
            status: dto.status,
        })
        .await?;

    Ok(Json(FeedbackResponse::from(feedback)))
}

#[utoipa::path(
    post,
    path = "/api/v1/feedback/{feedback_id}/replies",
    params(
        ("feedback_id" = Uuid, Path, description = "Feedback ID"),
    ),
    request_body = CreateFeedbackReplyDto,
    responses(
        (status = 201, description = "Reply created successfully", body = FeedbackReplyResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Feedback",
    security(("bearer_token" = []))
)]
pub async fn create_feedback_reply(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(feedback_id): Path<Uuid>,
    Json(dto): Json<CreateFeedbackReplyDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo: Arc<dyn FeedbackRepository> = Arc::new(PgFeedbackRepo::from(ctx.pool.clone()));
    let usecase = CreateFeedbackReplyUseCase::new(repo);

    let reply = usecase
        .execute(CreateFeedbackReplyInput {
            feedback_id,
            user_id: user.user_id,
            message: dto.message,
        })
        .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(FeedbackReplyResponse::from(reply)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/feedback/{feedback_id}/replies",
    params(
        ("feedback_id" = Uuid, Path, description = "Feedback ID"),
    ),
    responses(
        (status = 200, description = "List of replies retrieved successfully", body = [FeedbackReplyResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Feedback",
    security(("bearer_token" = []))
)]
pub async fn list_feedback_replies(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Path(feedback_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo: Arc<dyn FeedbackRepository> = Arc::new(PgFeedbackRepo::from(ctx.pool.clone()));
    let usecase = ListFeedbackRepliesUseCase::new(repo);

    let replies = usecase
        .execute(ListFeedbackRepliesInput {
            agency_id: ctx.agency.id,
            feedback_id,
        })
        .await?;

    let response_list: Vec<FeedbackReplyResponse> = replies.into_iter().map(|r| r.into()).collect();
    Ok(Json(response_list))
}
