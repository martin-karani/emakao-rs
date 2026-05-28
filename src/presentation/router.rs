use axum::{http::Method, middleware, Router};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::presentation::{
    app_state::AppState,
    http::routes::{
        accounting_routes, agency_routes, agreement_routes, analytics_routes, auth_routes,
        bank_reconciliation_routes, customisation_routes, dashboard_routes, disbursement_routes,
        document_routes, health_routes, insights_routes, inspection_routes, invoice_routes,
        ledger_routes, maintenance_routes, owner_routes, payment_routes, property_routes,
        resident_routes, staff_routes, subscription_routes, tax_routes, upload_routes,
        utility_routes, vendor_routes, webhook_routes, websocket_routes,
    },
    middleware::{
        admin_auth::require_admin,
        agency_context::resolve_agency_context,
        auth::require_auth,
        portal_guard::{
            caretaker_portal_guard, owner_portal_guard, resident_portal_guard, vendor_portal_guard,
        },
        subscription::subscription_middleware,
    },
    openapi::ApiDoc,
};

pub fn build_router(state: AppState) -> Router {
    let public_api = build_public_api(state.clone());
    let staff_api = build_staff_api(state.clone());
    let portal_api = build_portal_api(state.clone());
    let admin_api = build_admin_api(state.clone());

    // CORS: read allowed origins from config; fall back to permissive only in
    // debug builds. In production an empty `allowed_origins` list means the
    // server will reject cross-origin requests, which is the safe default.
    let cors = build_cors(&state);

    let mut router = Router::new()
        .merge(public_api)
        .merge(staff_api)
        .merge(portal_api)
        .merge(admin_api)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

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

// ── CORS ──────────────────────────────────────────────────────────────────────

fn build_cors(state: &AppState) -> CorsLayer {
    let origins = &state.config.allowed_origins;
    if origins.is_empty() {
        // No origins configured — deny all cross-origin requests in production.
        // In debug mode we allow everything for developer ergonomics.
        #[cfg(debug_assertions)]
        return CorsLayer::permissive();
        #[cfg(not(debug_assertions))]
        return CorsLayer::new();
    }

    let parsed: Vec<axum::http::HeaderValue> = origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(parsed))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ])
}

// ── Public (no auth) ──────────────────────────────────────────────────────────

fn build_public_api(state: AppState) -> Router<AppState> {
    // ISSUE 1 FIX: session_routes() (logout) was defined but never mounted.
    // Logout requires a valid JWT so it sits behind require_auth.
    let logout_route = auth_routes::session_routes(state.clone())
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Password change also needs auth — already handled inside change_password_routes().
    let pw_change = auth_routes::change_password_routes(state.clone());

    Router::new()
        .merge(health_routes::routes())
        .merge(webhook_routes::routes())
        .merge(websocket_routes::routes())
        .merge(subscription_routes::public_routes())
        .merge(auth_routes::staff_login_routes())
        .merge(auth_routes::resident_login_routes())
        .merge(auth_routes::owner_login_routes())
        .merge(auth_routes::vendor_login_routes())
        // ISSUE 5 FIX (part 1): caretaker login was implemented but never mounted.
        .merge(auth_routes::caretaker_login_routes())
        .merge(auth_routes::password_reset_routes())
        .merge(pw_change)
        .merge(logout_route)
}

// ── Staff ─────────────────────────────────────────────────────────────────────

fn build_staff_api(state: AppState) -> Router<AppState> {
    // ISSUE 9 FIX: billing_routes (initiate-payment, subscribe) were never mounted.
    // They must sit OUTSIDE subscription_middleware — a lapsed account must be
    // able to pay its way back to active status.
    let billing = subscription_routes::billing_routes();

    // Full middleware stack: auth → agency context → subscription gate.
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
        .merge(dashboard_routes::routes())
        .merge(document_routes::routes())
        .merge(analytics_routes::routes())
        .merge(insights_routes::routes())
        .merge(inspection_routes::routes())
        .merge(invoice_routes::routes())
        .merge(disbursement_routes::routes())
        .merge(upload_routes::routes())
        // Accounting + bank reconciliation
        .merge(accounting_routes::routes())
        .merge(bank_reconciliation_routes::routes())
        .merge(staff_routes::staff_routes())
        .merge(tax_routes::routes())
        // Customisation layer (settings, integrations, templates, workflow rules)
        .merge(customisation_routes::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            subscription_middleware,
        ))
        .layer(middleware::from_fn(crate::presentation::middleware::portal_guard::staff_portal_guard))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            resolve_agency_context,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Billing routes skip subscription_middleware but still need auth + agency context.
    let billing_authenticated = billing
        .layer(middleware::from_fn_with_state(
            state.clone(),
            resolve_agency_context,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Agency profile management — auth only, no agency-context middleware
    // (agency_id comes from the JWT claim directly in these handlers).
    // ISSUE 12 FIX: Added resolve_agency_context so suspended agencies cannot manage profiles.
    let agency_mgmt = agency_routes::management_routes()
        .layer(middleware::from_fn_with_state(
            state.clone(),
            resolve_agency_context,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .merge(operational)
        .merge(billing_authenticated)
        .merge(agency_mgmt)
}

// ── Portal (residents, owners, vendors, caretakers) ──────────────────────────

fn build_portal_api(state: AppState) -> Router<AppState> {
    let resident_api = resident_routes::resident_portal_routes()
        .layer(middleware::from_fn(resident_portal_guard));
    let owner_api = owner_routes::owner_portal_routes()
        .layer(middleware::from_fn(owner_portal_guard));
    let vendor_api = vendor_routes::vendor_portal_routes()
        .layer(middleware::from_fn(vendor_portal_guard));

    // ISSUE 5 FIX (part 2): caretaker portal routes were implemented but never
    // mounted. The caretaker_portal_guard enforces PortalType::Caretaker.
    let caretaker_api = maintenance_routes::caretaker_portal_routes()
        .layer(middleware::from_fn(caretaker_portal_guard));

    Router::new()
        .merge(resident_api)
        .merge(owner_api)
        .merge(vendor_api)
        .merge(caretaker_api)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            resolve_agency_context,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
}

// ── Admin (platform super-admin) ──────────────────────────────────────────────

fn build_admin_api(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(agency_routes::admin_routes())
        // ISSUE 8 FIX: admin subscription management routes were defined but
        // never merged into this router — platform admins couldn't manage plans.
        .merge(subscription_routes::admin_routes(state.clone()))
        .layer(middleware::from_fn_with_state(state, require_admin))
}
