// src/presentation/router.rs
//
// Application router — groups routes by domain boundary.
//
// ## Groups
//
// | Function              | Routes                                              |
// |-----------------------|-----------------------------------------------------|
// | `build_public_api`    | Health, webhooks, WS, public subscription,          |
// |                       | all portal login endpoints, forgot/reset-password   |
// | `build_staff_api`     | Operational staff routes + billing routes            |
// | `build_portal_api`    | Resident / Owner / Vendor / Caretaker portals        |
// | `build_admin_api`     | Platform-admin routes                                |
//
// All protected groups go through:
//   1. `require_auth`           — validates Bearer JWT + checks jti revocation blacklist
//   2. `resolve_agency_context` — loads agency row + tenant pool
//   3. `portal_guard`           — checks JWT portal claim matches the route group
//
// Staff routes additionally pass through `subscription_middleware`.

use axum::{middleware, Router};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use crate::{
    domain::enums::PortalType,
    presentation::{
        app_state::AppState,
        http::routes::{
            agency_routes, agreement_routes, ai_insights_routes, analytics_routes, auth_routes,
            dashboard_routes, disbursement_routes, document_routes, health_routes,
            inspection_routes, invoice_routes, ledger_routes, maintenance_routes, owner_routes,
            payment_routes, property_routes, resident_routes, subscription_routes, upload_routes,
            utility_routes, vendor_routes, webhook_routes, websocket_routes,
        },
        middleware::{
            admin_auth::require_admin, agency_context::resolve_agency_context, auth::require_auth,
            portal_guard::portal_guard, subscription::subscription_middleware,
        },
        openapi::ApiDoc,
    },
};

pub fn build_router(state: AppState) -> Router {
    let public_api = build_public_api(state.clone());
    let staff_api = build_staff_api(state.clone());
    let portal_api = build_portal_api(state.clone());
    let admin_api = build_admin_api(state.clone());

    let mut router = Router::new()
        .merge(public_api)
        .merge(staff_api)
        .merge(portal_api)
        .merge(admin_api)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Swagger UI — debug builds only
    #[cfg(debug_assertions)]
    {
        use utoipa::OpenApi;
        router = router.merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        );
    }

    router
}

// ── Public (no auth) ──────────────────────────────────────────────────────────

fn build_public_api(state: AppState) -> Router<AppState> {
    // change_password is auth-required but lives outside resolve_agency_context
    let pw_change = auth_routes::change_password_routes(state);

    Router::new()
        .merge(health_routes::routes())
        .merge(webhook_routes::routes())
        .merge(websocket_routes::routes())
        .merge(subscription_routes::public_routes())
        // ── Portal login endpoints ────────────────────────────────────────────
        .merge(auth_routes::staff_login_routes())
        .merge(auth_routes::resident_login_routes())
        .merge(auth_routes::owner_login_routes())
        .merge(auth_routes::vendor_login_routes())
        // ── Password management (no auth required for forgot/reset) ───────────
        .merge(auth_routes::password_reset_routes()) // POST /auth/forgot-password
        // POST /auth/reset-password
        .merge(pw_change)
}

// ── Staff ─────────────────────────────────────────────────────────────────────

fn build_staff_api(state: AppState) -> Router<AppState> {
    // ── Subscription-gated operational routes ─────────────────────────────────
    let operational = Router::new()
        // Core property management
        .merge(property_routes::routes())
        .merge(agreement_routes::routes())
        .merge(payment_routes::routes())
        .merge(ledger_routes::routes())
        .merge(utility_routes::routes())
        .merge(maintenance_routes::routes())
        .merge(resident_routes::staff_routes())
        .merge(owner_routes::staff_routes())
        .merge(vendor_routes::staff_routes())
        .merge(subscription_routes::agency_routes(state.clone()))
        // Dashboards & analytics
        .merge(dashboard_routes::routes()) // GET /api/v1/dashboard
        .merge(document_routes::routes()) // CRUD /api/v1/documents
        .merge(analytics_routes::routes()) // GET /api/v1/analytics/*
        .merge(ai_insights_routes::routes()) // GET /api/v1/ai/*
        .merge(inspection_routes::routes()) // CRUD /api/v1/inspections
        .merge(invoice_routes::routes()) // CRUD /api/v1/invoices
        .merge(disbursement_routes::routes()) // CRUD /api/v1/disbursements
        .merge(upload_routes::routes()) // POST /api/v1/upload
        .layer(middleware::from_fn_with_state(
            state.clone(),
            subscription_middleware,
        ));

    // Billing routes skip the subscription check (they ARE the payment path)
    let billing = subscription_routes::billing_routes();

    // Logout skips the agency-context resolution (it only needs the JWT)
    let session = auth_routes::session_routes(state.clone()); // POST /auth/logout

    with_auth_stack(
        Router::new()
            .merge(operational)
            .merge(billing)
            .merge(session),
        state,
        PortalType::Staff,
    )
}

// ── Portals ───────────────────────────────────────────────────────────────────

fn build_portal_api(state: AppState) -> Router<AppState> {
    Router::new()
        // Resident portal
        .merge(with_auth_stack(
            resident_routes::resident_portal_routes()
                .merge(maintenance_routes::resident_portal_routes()),
            state.clone(),
            PortalType::Resident,
        ))
        // Owner portal — disbursements are read-only from the owner's perspective
        .merge(with_auth_stack(
            owner_routes::owner_portal_routes().merge(disbursement_routes::owner_portal_routes()), // GET /portal/disbursements
            state.clone(),
            PortalType::Owner,
        ))
        // Vendor portal
        .merge(with_auth_stack(
            vendor_routes::vendor_portal_routes(),
            state.clone(),
            PortalType::Vendor,
        ))
        // Caretaker portal
        .merge(with_auth_stack(
            maintenance_routes::caretaker_portal_routes(),
            state.clone(),
            PortalType::Caretaker,
        ))
}

// ── Admin ─────────────────────────────────────────────────────────────────────

fn build_admin_api(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(agency_routes::admin_routes())
        .merge(subscription_routes::admin_routes(state.clone()))
        .layer(middleware::from_fn_with_state(state, require_admin))
}

// ── Helper: standard auth stack ───────────────────────────────────────────────

/// Wraps `routes` with the three-layer authentication stack:
///
///   require_auth  (JWT decode + **jti blacklist check**)
///       ↓
///   resolve_agency_context  (load agency row + open tenant pool)
///       ↓
///   portal_guard  (assert JWT portal_type == expected)
///
/// Layers are applied innermost-first in Axum, so the outermost
/// `.layer()` call here runs first at request time.
fn with_auth_stack(
    routes: Router<AppState>,
    state: AppState,
    portal: PortalType,
) -> Router<AppState> {
    routes
        .layer(middleware::from_fn(move |req, next| {
            portal_guard(portal, req, next)
        }))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            resolve_agency_context,
        ))
        .layer(middleware::from_fn_with_state(state, require_auth))
}
