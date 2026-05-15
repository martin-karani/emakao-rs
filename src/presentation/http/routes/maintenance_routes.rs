use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::maintenance::{
        // ── Caretaker portal
        caretaker_create_work_order,
        caretaker_list_work_orders,
        // ── Caretakers (staff manages them)
        create_caretaker,
        // ── Work orders — staff
        create_work_order,
        // ── Comments — staff
        create_work_order_comment,
        get_work_order,
        // ── Activity log — staff only
        get_work_order_activity,
        get_work_order_by_code,
        list_caretakers,
        list_comment_replies,
        list_work_order_comments,
        list_work_orders,
        // ── Resident portal
        resident_create_work_order,
        resident_create_work_order_comment,
        resident_list_work_order_comments,
        resident_list_work_orders,
        update_caretaker,
        update_work_order,
    },
};

// ── Staff routes ──────────────────────────────────────────────────────────────
//
// Mounted inside `build_staff_api` which wraps the whole set with:
//   require_auth → resolve_agency_context → portal_guard(Staff) → subscription_middleware

pub fn routes() -> Router<AppState> {
    Router::new()
        // ── Work orders ───────────────────────────────────────────────────────
        // IMPORTANT: `/by-code/:code` must be declared before `/:id` so axum
        // does not capture the literal "by-code" segment as a UUID.
        .route(
            "/api/v1/work-orders/by-code/:code",
            get(get_work_order_by_code),
        )
        .route(
            "/api/v1/work-orders",
            get(list_work_orders).post(create_work_order),
        )
        .route(
            "/api/v1/work-orders/:id",
            get(get_work_order).patch(update_work_order),
        )
        // ── Comments
        .route(
            "/api/v1/work-orders/:id/comments",
            get(list_work_order_comments).post(create_work_order_comment),
        )
        .route(
            "/api/v1/work-orders/:id/comments/:comment_id/replies",
            get(list_comment_replies),
        )
        // ── Activity log
        .route(
            "/api/v1/work-orders/:id/activity",
            get(get_work_order_activity),
        )
        // ── Caretaker management (CRUD owned by staff)
        .route(
            "/api/v1/caretakers",
            get(list_caretakers).post(create_caretaker),
        )
        .route("/api/v1/caretakers/:id", patch(update_caretaker))
}

// ── Caretaker portal routes ───────────────────────────────────────────────────
//
// Mounted inside `build_portal_api` with:
//   require_auth → resolve_agency_context → portal_guard(Caretaker)
//
// Caretakers can:
//   • See and submit work orders for their property
//   • Comment on work orders (is_internal forced to false by use case)
// Caretakers cannot:
//   • Manage other caretakers
//   • Access the activity log
//   • Set cost / vendor / internal_notes fields (enforced by use case)

pub fn caretaker_portal_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/caretaker/work-orders",
            get(caretaker_list_work_orders).post(caretaker_create_work_order),
        )
        .route(
            "/api/v1/caretaker/work-orders/:id",
            get(get_work_order), // re-uses staff handler — same read logic
        )
        .route(
            "/api/v1/caretaker/work-orders/:id/comments",
            get(list_work_order_comments) // sees internal; caretakers are trusted actors
                .post(create_work_order_comment),
        )
        .route(
            "/api/v1/caretaker/work-orders/:id/comments/:comment_id/replies",
            get(list_comment_replies),
        )
}

// ── Resident portal routes ────────────────────────────────────────────────────
//
// Merged into the existing `resident_routes::resident_portal_routes()` set inside
// `build_portal_api` with:
//   require_auth → resolve_agency_context → portal_guard(Resident)
//
// Residents can:
//   • See tenant-visible work orders for their unit
//   • Submit new work orders (restricted fields; public response returned)
//   • Comment on tenant-visible work orders (never internal)
// Residents cannot:
//   • See internal comments, cost fields, internal_notes, or the activity log

pub fn resident_portal_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/resident/work-orders",
            get(resident_list_work_orders).post(resident_create_work_order),
        )
        .route(
            // Residents get WorkOrderPublicResponse from the use-case layer;
            // re-using the staff get handler is intentional — the response
            // mapping (WorkOrderPublicResponse vs WorkOrderResponse) is the
            // only difference and that lives in the portal-specific handlers.
            "/api/v1/resident/work-orders/:id",
            get(resident_list_work_orders), // scoped list doubles as detail via filter
        )
        .route(
            "/api/v1/resident/work-orders/:id/comments",
            get(resident_list_work_order_comments).post(resident_create_work_order_comment),
        )
        .route(
            "/api/v1/resident/work-orders/:id/comments/:comment_id/replies",
            get(list_comment_replies), // replies follow same include_internal=false in use case
        )
}
