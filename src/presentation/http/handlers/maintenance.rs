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
        use_cases::maintenance::{
            add_work_order_comment::{AddWorkOrderCommentInput, AddWorkOrderCommentUseCase},
            create_caretaker::{CreateCaretakerInput, CreateCaretakerUseCase},
            create_work_order::{CreateWorkOrderInput, CreateWorkOrderUseCase},
            get_work_order::{
                GetWorkOrderByCodeUseCase, GetWorkOrderUseCase, GetWorkOrdersForCaretakerUseCase,
                GetWorkOrdersForResidentUseCase,
            },
            get_work_order_activity::GetWorkOrderActivityUseCase,
            list_caretakers::ListCaretakersUseCase,
            list_work_order_comments::{
                ListCommentRepliesUseCase, ListWorkOrderCommentsInput, ListWorkOrderCommentsUseCase,
            },
            list_work_orders::{ListWorkOrdersInput, ListWorkOrdersUseCase},
            update_caretaker::{UpdateCaretakerInput, UpdateCaretakerUseCase},
            update_work_order::{UpdateWorkOrderInput, UpdateWorkOrderUseCase},
        },
    },
    domain::{
        auth::AuthenticatedUser,
        enums::{WorkOrderCommentAuthorType, WorkOrderReporterType},
    },
    infrastructure::db::maintenance_repository_sqlx::PgMaintenanceRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{dto::maintenance::*, responses::maintenance::*},
        require_feature,
    },
};

// ── Shared repo macro ─────────────────────────────────────────────────────────
// Moves ctx.pool — store agency_id separately before calling this.

macro_rules! repo {
    ($ctx:expr) => {
        Arc::new(PgMaintenanceRepo::from($ctx.pool))
    };
}

// ─────────────────────────────────────────────────────────────────────────────
// CARETAKERS
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/caretakers",
    params(ListCaretakersParams),
    responses(
        (status = 200, body = Vec<CaretakerResponse>),
        (status = 401, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn list_caretakers(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Query(params): Query<ListCaretakersParams>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let items = ListCaretakersUseCase::new(repo!(ctx))
        .execute(
            params.property_id,
            params.limit.unwrap_or(50),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(CaretakerResponse::from)
            .collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    post, path = "/api/v1/caretakers",
    request_body = CreateCaretakerDto,
    responses(
        (status = 201, body = CaretakerResponse),
        (status = 401, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn create_caretaker(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateCaretakerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let ct = CreateCaretakerUseCase::new(repo!(ctx))
        .execute(CreateCaretakerInput {
            property_id: dto.property_id,
            created_by: user.user_id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            phone: dto.phone,
            email: dto.email,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(CaretakerResponse::from(ct))))
}

#[utoipa::path(
    patch, path = "/api/v1/caretakers/{id}",
    params(("id" = Uuid, Path, description = "Caretaker UUID")),
    request_body = UpdateCaretakerDto,
    responses(
        (status = 200, body = CaretakerResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn update_caretaker(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateCaretakerDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let ct = UpdateCaretakerUseCase::new(repo!(ctx))
        .execute(UpdateCaretakerInput {
            id,
            first_name: dto.first_name,
            last_name: dto.last_name,
            phone: dto.phone,
            email: dto.email,
            is_active: dto.is_active,
        })
        .await?;

    Ok(Json(CaretakerResponse::from(ct)))
}

// ─────────────────────────────────────────────────────────────────────────────
// WORK ORDERS — staff routes
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/work-orders",
    params(ListWorkOrdersParams),
    responses(
        (status = 200, body = Vec<WorkOrderResponse>),
        (status = 401, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn list_work_orders(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Query(params): Query<ListWorkOrdersParams>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let items = ListWorkOrdersUseCase::new(repo!(ctx))
        .execute(ListWorkOrdersInput {
            agency_id,
            property_id: params.property_id,
            unit_id: params.unit_id,
            status: params.status,
            priority: params.priority,
            category: params.category,
            reporter_type: params.reporter_type,
            assigned_caretaker_id: params.assigned_caretaker_id,
            limit: params.limit.unwrap_or(20),
            offset: params.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderResponse::from)
            .collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/work-orders/{id}",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    responses(
        (status = 200, body = WorkOrderResponse),
        (status = 404, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn get_work_order(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let order = GetWorkOrderUseCase::new(repo!(ctx))
        .execute(agency_id, id)
        .await?;
    Ok(Json(WorkOrderResponse::from(order)))
}

#[utoipa::path(
    get, path = "/api/v1/work-orders/by-code/{code}",
    params(("code" = String, Path, description = "Work order code, e.g. MGRD-0042")),
    responses(
        (status = 200, body = WorkOrderResponse),
        (status = 404, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn get_work_order_by_code(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let order = GetWorkOrderByCodeUseCase::new(repo!(ctx))
        .execute(agency_id, &code)
        .await?;
    Ok(Json(WorkOrderResponse::from(order)))
}

#[utoipa::path(
    post, path = "/api/v1/work-orders",
    request_body = CreateWorkOrderDto,
    responses(
        (status = 201, body = WorkOrderResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn create_work_order(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");
    if dto.vendor_id.is_some() {
        require_feature!(state, agency_id, "maint_vendor_portal");
    }

    let uc = CreateWorkOrderUseCase::new(repo!(ctx), state.notifications.clone());
    let order = uc
        .execute(CreateWorkOrderInput {
            agency_id,
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            reported_by: user.user_id,
            reporter_type: dto.reporter_type.unwrap_or(WorkOrderReporterType::Staff),
            reporter_resident_id: dto.reporter_resident_id,
            reporter_caretaker_id: dto.reporter_caretaker_id,
            title: dto.title,
            description: dto.description,
            category: dto.category,
            priority: dto.priority,
            vendor_id: dto.vendor_id,
            assigned_to: dto.assigned_to,
            assigned_caretaker_id: dto.assigned_caretaker_id,
            due_date: dto.due_date,
            scheduled_at: dto.scheduled_at,
            estimated_cost_kes: dto.estimated_cost_kes,
            is_tenant_visible: dto.is_tenant_visible.unwrap_or(true),
            internal_notes: dto.internal_notes,
            attachments: dto.attachments.unwrap_or_default(),
            notify_resident_email: None,
            notify_resident_phone: None,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(WorkOrderResponse::from(order))))
}

#[utoipa::path(
    patch, path = "/api/v1/work-orders/{id}",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    request_body = UpdateWorkOrderDto,
    responses(
        (status = 200, body = WorkOrderResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn update_work_order(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");
    if dto.vendor_id.as_ref().and_then(|v| v.as_ref()).is_some() {
        require_feature!(state, agency_id, "maint_vendor_portal");
    }

    let uc = UpdateWorkOrderUseCase::new(repo!(ctx), state.notifications.clone());
    let order = uc
        .execute(UpdateWorkOrderInput {
            agency_id,
            work_order_id: id,
            actor_id: user.user_id,
            actor_type: "staff".to_string(),
            status: dto.status,
            priority: dto.priority,
            category: dto.category,
            vendor_id: dto.vendor_id,
            assigned_to: dto.assigned_to,
            assigned_caretaker_id: dto.assigned_caretaker_id,
            description: dto.description,
            internal_notes: dto.internal_notes,
            due_date: dto.due_date,
            scheduled_at: dto.scheduled_at,
            started_at: dto.started_at,
            completed_at: dto.completed_at,
            estimated_cost_kes: dto.estimated_cost_kes,
            actual_cost_kes: dto.actual_cost_kes,
            is_tenant_visible: dto.is_tenant_visible,
            attachments: dto.attachments,
            notify_resident_email: None,
            notify_resident_phone: None,
        })
        .await?;

    Ok(Json(WorkOrderResponse::from(order)))
}

// ─────────────────────────────────────────────────────────────────────────────
// PORTAL ROUTES — caretaker + resident
// ─────────────────────────────────────────────────────────────────────────────

pub async fn caretaker_create_work_order(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let uc = CreateWorkOrderUseCase::new(repo!(ctx), state.notifications.clone());
    let order = uc
        .execute(CreateWorkOrderInput {
            agency_id,
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            reported_by: user.user_id,
            reporter_type: WorkOrderReporterType::Caretaker,
            reporter_resident_id: None,
            reporter_caretaker_id: dto.reporter_caretaker_id,
            title: dto.title,
            description: dto.description,
            category: dto.category,
            priority: dto.priority,
            vendor_id: None,
            assigned_to: None,
            assigned_caretaker_id: None,
            due_date: dto.due_date,
            scheduled_at: dto.scheduled_at,
            estimated_cost_kes: None,
            is_tenant_visible: dto.is_tenant_visible.unwrap_or(true),
            internal_notes: None,
            attachments: dto.attachments.unwrap_or_default(),
            notify_resident_email: None,
            notify_resident_phone: None,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(WorkOrderResponse::from(order))))
}

pub async fn caretaker_list_work_orders(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Query(params): Query<ListWorkOrdersParams>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let caretaker_id = params.assigned_caretaker_id.ok_or_else(|| {
        AppError::Validation("assigned_caretaker_id is required on this route".into())
    })?;

    let items = GetWorkOrdersForCaretakerUseCase::new(repo!(ctx))
        .execute(
            caretaker_id,
            params.limit.unwrap_or(20),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderResponse::from)
            .collect::<Vec<_>>(),
    ))
}

pub async fn resident_create_work_order(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateWorkOrderDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let uc = CreateWorkOrderUseCase::new(repo!(ctx), state.notifications.clone());
    let order = uc
        .execute(CreateWorkOrderInput {
            agency_id,
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            reported_by: user.user_id,
            reporter_type: WorkOrderReporterType::Resident,
            reporter_resident_id: dto.reporter_resident_id,
            reporter_caretaker_id: None,
            title: dto.title,
            description: dto.description,
            category: dto.category,
            priority: dto.priority,
            vendor_id: None,
            assigned_to: None,
            assigned_caretaker_id: None,
            due_date: None,
            scheduled_at: None,
            estimated_cost_kes: None,
            is_tenant_visible: true,
            internal_notes: None,
            attachments: dto.attachments.unwrap_or_default(),
            notify_resident_email: None,
            notify_resident_phone: None,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(WorkOrderPublicResponse::from(order)),
    ))
}

pub async fn resident_list_work_orders(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Query(params): Query<ListWorkOrdersParams>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let resident_id = params.reporter_resident_id.ok_or_else(|| {
        AppError::Validation("reporter_resident_id is required on this route".into())
    })?;

    let items = GetWorkOrdersForResidentUseCase::new(repo!(ctx))
        .execute(
            resident_id,
            params.limit.unwrap_or(20),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderPublicResponse::from)
            .collect::<Vec<_>>(),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// COMMENTS
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/work-orders/{id}/comments",
    params(
        ("id" = Uuid, Path, description = "Work order UUID"),
        ListWorkOrderCommentsParams,
    ),
    responses(
        (status = 200, body = Vec<WorkOrderCommentResponse>),
        (status = 404, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn list_work_order_comments(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
    Query(params): Query<ListWorkOrderCommentsParams>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let items = ListWorkOrderCommentsUseCase::new(repo!(ctx))
        .execute(ListWorkOrderCommentsInput {
            agency_id,
            work_order_id: id,
            include_internal: true,
            top_level_only: params.top_level_only.unwrap_or(false),
            limit: params.limit.unwrap_or(50),
            offset: params.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderCommentResponse::from)
            .collect::<Vec<_>>(),
    ))
}

pub async fn resident_list_work_order_comments(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
    Query(params): Query<ListWorkOrderCommentsParams>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let items = ListWorkOrderCommentsUseCase::new(repo!(ctx))
        .execute(ListWorkOrderCommentsInput {
            agency_id,
            work_order_id: id,
            include_internal: false,
            top_level_only: params.top_level_only.unwrap_or(false),
            limit: params.limit.unwrap_or(50),
            offset: params.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderCommentResponse::from)
            .collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/work-orders/{id}/comments/{comment_id}/replies",
    params(
        ("id" = Uuid, Path, description = "Work order UUID"),
        ("comment_id" = Uuid, Path, description = "Parent comment UUID"),
    ),
    responses((status = 200, body = Vec<WorkOrderCommentResponse>)),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn list_comment_replies(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path((work_order_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let items = ListCommentRepliesUseCase::new(repo!(ctx))
        .execute(agency_id, work_order_id, comment_id, true, 100, 0)
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderCommentResponse::from)
            .collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    post, path = "/api/v1/work-orders/{id}/comments",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    request_body = CreateWorkOrderCommentDto,
    responses(
        (status = 201, body = WorkOrderCommentResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn create_work_order_comment(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(work_order_id): Path<Uuid>,
    Json(dto): Json<CreateWorkOrderCommentDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let comment = AddWorkOrderCommentUseCase::new(repo!(ctx))
        .execute(AddWorkOrderCommentInput {
            agency_id,
            work_order_id,
            parent_comment_id: dto.parent_comment_id,
            author_id: user.user_id,
            author_type: dto.author_type.unwrap_or(WorkOrderCommentAuthorType::Staff),
            author_resident_id: dto.author_resident_id,
            author_caretaker_id: dto.author_caretaker_id,
            body: dto.body,
            is_internal: dto.is_internal.unwrap_or(false),
            attachments: dto.attachments.unwrap_or_default(),
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(WorkOrderCommentResponse::from(comment)),
    ))
}

pub async fn resident_create_work_order_comment(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(work_order_id): Path<Uuid>,
    Json(dto): Json<CreateWorkOrderCommentDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let comment = AddWorkOrderCommentUseCase::new(repo!(ctx))
        .execute(AddWorkOrderCommentInput {
            agency_id,
            work_order_id,
            parent_comment_id: dto.parent_comment_id,
            author_id: user.user_id,
            author_type: WorkOrderCommentAuthorType::Resident,
            author_resident_id: dto.author_resident_id,
            author_caretaker_id: None,
            body: dto.body,
            is_internal: false,
            attachments: dto.attachments.unwrap_or_default(),
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(WorkOrderCommentResponse::from(comment)),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// ACTIVITY LOG
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/work-orders/{id}/activity",
    params(("id" = Uuid, Path, description = "Work order UUID")),
    responses(
        (status = 200, body = Vec<WorkOrderActivityResponse>),
        (status = 404, body = ErrorResponse),
    ),
    tag = "Maintenance", security(("bearer_token" = []))
)]
pub async fn get_work_order_activity(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let agency_id = ctx.agency.id;
    require_feature!(state, agency_id, "maint_work_orders");

    let items = GetWorkOrderActivityUseCase::new(repo!(ctx))
        .execute(agency_id, id, 200, 0)
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(WorkOrderActivityResponse::from)
            .collect::<Vec<_>>(),
    ))
}
