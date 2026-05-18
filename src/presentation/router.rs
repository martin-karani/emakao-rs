use axum::{middleware, Router};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use crate::{
    domain::enums::PortalType,
    presentation::{
        app_state::AppState,
        http::routes::{
            accounting_routes, agency_routes, agreement_routes, analytics_routes, auth_routes,
            bank_reconciliation_routes, dashboard_routes, disbursement_routes, document_routes,
            health_routes, insights_routes, inspection_routes, invoice_routes, ledger_routes,
            maintenance_routes, owner_routes, payment_routes, property_routes, resident_routes,
            staff_routes, subscription_routes, upload_routes, utility_routes, vendor_routes,
            webhook_routes, websocket_routes,
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
        // Accounting + bank reconciliation (Growth+ — further gated per-handler
        // via the `acct_double_entry` feature entitlement check)
        .merge(accounting_routes::routes())
        .merge(bank_reconciliation_routes::routes())
        .merge(staff_routes::staff_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            subscription_middleware,
        ));

    let billing = subscription_routes::billing_routes();
    let session = auth_routes::session_routes(state.clone());

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
        .merge(with_auth_stack(
            resident_routes::resident_portal_routes()
                .merge(maintenance_routes::resident_portal_routes()),
            state.clone(),
            PortalType::Resident,
        ))
        .merge(with_auth_stack(
            owner_routes::owner_portal_routes().merge(disbursement_routes::owner_portal_routes()),
            state.clone(),
            PortalType::Owner,
        ))
        .merge(with_auth_stack(
            vendor_routes::vendor_portal_routes(),
            state.clone(),
            PortalType::Vendor,
        ))
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

// ── Helper ────────────────────────────────────────────────────────────────────

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
