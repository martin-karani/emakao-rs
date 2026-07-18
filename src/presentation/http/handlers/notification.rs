// src/presentation/http/handlers/notification.rs
//
//   GET   /api/notifications         — list latest notifications for the caller
//   PATCH /api/notifications/read-all — mark all as read
//   PATCH /api/notifications/:id/read — mark one as read

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::notification::{
            list_notifications::{ListNotificationsInput, ListNotificationsUseCase},
            mark_all_notifications_read::{
                MarkAllNotificationsReadInput, MarkAllNotificationsReadUseCase,
            },
            mark_notification_read::{MarkNotificationReadInput, MarkNotificationReadUseCase},
        },
        ports::notification_repository::NotificationRepository,
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::notification_repository_sqlx::PgNotificationRepo,
    presentation::{
        app_state::AppState,
        extractors::AgencyContext,
        http::{
            dto::notification::ListNotificationsQuery,
            responses::notification::{NotificationListResponse, NotificationResponse},
        },
    },
};

/// GET /api/notifications
pub async fn list_notifications(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(q): Query<ListNotificationsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgNotificationRepo::new(ctx.pool.clone()));

    let notifications = ListNotificationsUseCase::new(repo.clone())
        .execute(ListNotificationsInput {
            user_id: user.user_id,
            limit: q.limit,
        })
        .await?;

    let unread_count = repo.unread_count(user.user_id).await?;

    Ok(Json(NotificationListResponse {
        notifications: notifications.into_iter().map(NotificationResponse::from).collect(),
        unread_count,
    }))
}

/// PATCH /api/notifications/:id/read
pub async fn mark_notification_read(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgNotificationRepo::new(ctx.pool));

    MarkNotificationReadUseCase::new(repo)
        .execute(MarkNotificationReadInput {
            id,
            user_id: user.user_id,
        })
        .await?;

    Ok(StatusCode::OK)
}

/// PATCH /api/notifications/read-all
pub async fn mark_all_notifications_read(
    State(_state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgNotificationRepo::new(ctx.pool));

    MarkAllNotificationsReadUseCase::new(repo)
        .execute(MarkAllNotificationsReadInput {
            user_id: user.user_id,
        })
        .await?;

    Ok(StatusCode::OK)
}
