use axum::{routing::get, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::property::{
        create_property, delete_property, get_property, get_property_billing, get_property_by_slug,
        list_properties, update_property, upsert_property_billing,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/properties",
            get(list_properties).post(create_property),
        )
        .route(
            "/api/v1/properties/{id}",
            get(get_property)
                .put(update_property)
                .delete(delete_property),
        )
        .route(
            "/api/v1/properties/by-slug/{slug}",
            get(get_property_by_slug),
        )
        .route(
            "/api/v1/properties/{id}/billing",
            get(get_property_billing).put(upsert_property_billing),
        )
}
