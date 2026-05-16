// src/presentation/http/routes/disbursement_routes.rs

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::disbursement::{
        create_disbursement, get_disbursement, initiate_payout, list_disbursements,
        update_disbursement_status,
    },
};

/// Staff routes — full CRUD + payout initiation.
/// Protected by the auth stack in router.rs (Staff portal).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/disbursements",
            get(list_disbursements).post(create_disbursement),
        )
        .route("/api/v1/disbursements/:id", get(get_disbursement))
        .route(
            "/api/v1/disbursements/:id/status",
            patch(update_disbursement_status),
        )
        .route(
            "/api/v1/disbursements/:id/initiate-payout",
            post(initiate_payout),
        )
}

/// Owner portal routes — read-only view of their own disbursements.
/// The handler filters by `owner_id` extracted from the JWT so an owner
/// cannot see another owner's disbursements.
///
/// Mounted separately in `build_portal_api` under PortalType::Owner.
pub fn owner_portal_routes() -> Router<AppState> {
    Router::new()
        .route("/portal/disbursements", get(list_my_disbursements))
        .route("/portal/disbursements/:id", get(get_my_disbursement))
}

// ── Owner-portal-specific handlers ───────────────────────────────────────────
// Thin wrappers that scope the query to the authenticated owner's ID.

use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::disbursement::{
            get_disbursement::GetDisbursementUseCase,
            list_disbursements::{ListDisbursementsInput, ListDisbursementsUseCase},
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::disbursement_repository_sqlx::PgDisbursementRepo,
    presentation::{
        extractors::AgencyContext,
        http::{
            handlers::disbursement::ListDisbursementsParams,
            responses::disbursement::DisbursementResponse,
        },
    },
};

/// GET /portal/disbursements
/// Lists disbursements that belong to the calling owner.
async fn list_my_disbursements(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(p): Query<ListDisbursementsParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = std::sync::Arc::new(PgDisbursementRepo::new(ctx.pool));
    let disbursements = ListDisbursementsUseCase::new(repo)
        .execute(ListDisbursementsInput {
            agency_id: ctx.agency.id,
            // Force filter to the JWT owner — ignores any owner_id in query params
            owner_id: Some(user.user_id),
            property_id: p.property_id,
            status: p.status,
            limit: p.limit,
            offset: p.offset,
        })
        .await?;

    Ok(Json(
        disbursements
            .into_iter()
            .map(DisbursementResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /portal/disbursements/:id
async fn get_my_disbursement(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = std::sync::Arc::new(PgDisbursementRepo::new(ctx.pool));
    let d = GetDisbursementUseCase::new(repo)
        .execute(ctx.agency.id, id)
        .await?;

    // Ownership check — owners can only see their own disbursements
    if d.owner_id != user.user_id {
        return Err(AppError::NotFound(format!("disbursement {id}")));
    }

    Ok(Json(DisbursementResponse::from(d)))
}
