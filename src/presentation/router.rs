use axum::{middleware, Router};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

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
        admin_auth::require_admin, agency_context::resolve_agency_context, auth::require_auth,
        portal_guard::{owner_portal_guard, resident_portal_guard, vendor_portal_guard},
        subscription::subscription_middleware,
    },
    openapi::ApiDoc,
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
    let pw_change = auth_routes::change_password_routes(state);

    Router::new()
        .merge(health_routes::routes())
        .merge(webhook_routes::routes())
        .merge(websocket_routes::routes())
        .merge(subscription_routes::public_routes())
        .merge(auth_routes::staff_login_routes())
        .merge(auth_routes::resident_login_routes())
        .merge(auth_routes::owner_login_routes())
        .merge(auth_routes::vendor_login_routes())
        .merge(auth_routes::password_reset_routes())
        .merge(pw_change)
}

// ── Staff ─────────────────────────────────────────────────────────────────────

fn build_staff_api(state: AppState) -> Router<AppState> {
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
        .layer(middleware::from_fn_with_state(
            state.clone(),
            resolve_agency_context,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Agency profile management — auth only, no agency-context middleware
    // (agency_id comes from the JWT claim directly in these handlers).
    let agency_mgmt = agency_routes::management_routes()
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new().merge(operational).merge(agency_mgmt)
}

// ── Portal (residents, owners, vendors) ──────────────────────────────────────

fn build_portal_api(state: AppState) -> Router<AppState> {
    let resident_api = resident_routes::resident_portal_routes()
        .layer(middleware::from_fn(resident_portal_guard));
    let owner_api = owner_routes::owner_portal_routes()
        .layer(middleware::from_fn(owner_portal_guard));
    let vendor_api = vendor_routes::vendor_portal_routes()
        .layer(middleware::from_fn(vendor_portal_guard));

    Router::new()
        .merge(resident_api)
        .merge(owner_api)
        .merge(vendor_api)
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
        .layer(middleware::from_fn_with_state(state, require_admin))
}
