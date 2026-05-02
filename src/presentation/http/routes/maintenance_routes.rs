use axum::{routing::get, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::maintenance::{
        create_work_order, get_work_order, list_work_orders, update_work_order,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/work-orders",
            get(list_work_orders).post(create_work_order),
        )
        .route(
            "/api/v1/work-orders/:id",
            get(get_work_order).patch(update_work_order),
        )
}
