//! Application router — groups routes by domain boundary.
//!
//! ## Groups
//!
//! | Function              | Routes                                        |
//! |-----------------------|-----------------------------------------------|
//! | `build_public_api`    | Health, webhooks, WS, public subscription,    |
//! |                       | all portal login endpoints                     |
//! | `build_staff_api`     | Operational staff routes + billing routes      |
//! | `build_portal_api`    | Resident / Owner / Vendor portal routes        |
//! | `build_admin_api`     | Platform-admin routes                          |
//!
//! All protected groups go through:
//!   1. `require_auth`          — validates Bearer JWT
//!   2. `resolve_agency_context`— loads agency row + tenant pool
//!   3. `portal_guard`          — checks JWT portal claim matches the route group
//!
//! Staff routes additionally pass through `subscription_middleware`.

use axum::{middleware, Router};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use crate::{
    domain::enums::PortalType,
    presentation::{
        app_state::AppState,
        http::routes::{
            agency_routes, agreement_routes, auth_routes, health_routes, ledger_routes,
            maintenance_routes, owner_routes, payment_routes, property_routes, resident_routes,
            subscription_routes, utility_routes, vendor_routes, webhook_routes, websocket_routes,
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

// ── Public (no auth) ─────────────────────────────────────────────────────────

fn build_public_api(state: AppState) -> Router<AppState> {
    // Change-password is auth-required but lives outside the
    // resolve_agency_context stack, so it is wired here.
    let pw_change = auth_routes::change_password_routes(state);

    Router::new()
        .merge(health_routes::routes())
        .merge(webhook_routes::routes())
        .merge(websocket_routes::routes())
        .merge(subscription_routes::public_routes())
        // Login endpoints for every portal type
        .merge(auth_routes::staff_login_routes())
        .merge(auth_routes::resident_login_routes())
        .merge(auth_routes::owner_login_routes())
        .merge(auth_routes::vendor_login_routes())
        .merge(pw_change)
}

// ── Staff ─────────────────────────────────────────────────────────────────────

fn build_staff_api(state: AppState) -> Router<AppState> {
    let operational = Router::new()
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
        .layer(middleware::from_fn_with_state(
            state.clone(),
            subscription_middleware,
        ));

    // Billing routes skip the subscription check (they are the payment path)
    let billing = subscription_routes::billing_routes();

    with_auth_stack(
        Router::new().merge(operational).merge(billing),
        state,
        PortalType::Staff,
    )
}

// ── Portals ───────────────────────────────────────────────────────────────────

fn build_portal_api(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(with_auth_stack(
            resident_routes::resident_portal_routes(),
            state.clone(),
            PortalType::Resident,
        ))
        .merge(with_auth_stack(
            owner_routes::owner_portal_routes(),
            state.clone(),
            PortalType::Owner,
        ))
        .merge(with_auth_stack(
            vendor_routes::vendor_portal_routes(),
            state.clone(),
            PortalType::Vendor,
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

/// Wrap `routes` with the three-layer authentication stack:
///   require_auth → resolve_agency_context → portal_guard
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
