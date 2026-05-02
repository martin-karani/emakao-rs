use axum::{routing::get, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::property::{
        create_property, delete_property, get_property, list_properties, update_property,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/properties",
            get(list_properties).post(create_property),
        )
        .route(
            "/api/v1/properties/:id",
            get(get_property)
                .put(update_property)
                .delete(delete_property),
        )
}
