use axum::{middleware, Router};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    domain::auth::PortalType,
    presentation::{
        app_state::AppState,
        http::routes::{
            agency_routes, agreement_routes, auth_routes, health_routes, ledger_routes,
            maintenance_routes, owner_routes, payment_routes, property_routes, resident_routes,
            subscription_routes, utility_routes, vendor_routes, webhook_routes, websocket_routes,
        },
        middleware::{
            admin_auth::require_admin, auth::require_auth, portal_guard::portal_guard,
            subscription::subscription_middleware, agency_context::resolve_agency_context,
        },
        openapi::ApiDoc,
    },
};

pub fn build_router(state: AppState) -> Router {
    // Infrastructure (no auth)
    let infra = health_routes::routes();
    let webhooks = webhook_routes::routes();
    let ws = websocket_routes::routes();

    // Public routes
    let public_sub = subscription_routes::public_routes();

    // Auth routes (no resolve_agency_context; agencies resolved inside use cases)
    let staff_auth = auth_routes::staff_login_routes();
    let resident_auth = auth_routes::resident_login_routes();
    let owner_auth = auth_routes::owner_login_routes();
    let vendor_auth = auth_routes::vendor_login_routes();
    let pw_change = auth_routes::change_password_routes(state.clone());

    // Helper: wrap routes in auth+tenant+portal stack
    let s = state.clone();
    let protected = |routes: Router<AppState>, portal: PortalType| {
        routes
            .layer(middleware::from_fn(move |req, next| {
                portal_guard(portal, req, next)
            }))
            .layer(middleware::from_fn_with_state(s.clone(), resolve_agency_context))
            .layer(middleware::from_fn_with_state(s.clone(), require_auth))
    };

    // Staff API (property management + subscription check)
    let staff_operational = Router::new()
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
    let staff_api = protected(staff_operational, PortalType::Staff);

    // Billing routes (auth+tenant, no subscription check)
    let billing_api = protected(subscription_routes::billing_routes(), PortalType::Staff);

    // Portal APIs
    let resident_api = protected(
        resident_routes::resident_portal_routes(),
        PortalType::Resident,
    );
    let owner_api = protected(owner_routes::owner_portal_routes(), PortalType::Owner);
    let vendor_api = protected(vendor_routes::vendor_portal_routes(), PortalType::Vendor);

    // Platform admin
    let admin_api = Router::new()
        .merge(agency_routes::admin_routes())
        .merge(subscription_routes::admin_routes(state.clone()))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin));

    let public_api = Router::new()
        .merge(infra)
        .merge(webhooks)
        .merge(ws)
        .merge(public_sub)
        .merge(staff_auth)
        .merge(resident_auth)
        .merge(owner_auth)
        .merge(vendor_auth);

    let protected_api = Router::new()
        .merge(pw_change)
        .merge(staff_api)
        .merge(billing_api)
        .merge(resident_api)
        .merge(owner_api)
        .merge(vendor_api);

    let mut router = Router::new()
        .merge(public_api)
        .merge(protected_api)
        .merge(admin_api)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Swagger UI (debug only)
    #[cfg(debug_assertions)]
    {
        use utoipa::OpenApi;
        router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    }

    router
}
