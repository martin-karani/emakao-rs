use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::presentation::app_state::AppState;
use crate::presentation::http::handlers::checklist::{
    attach_checklist_to_property, create_checklist, create_checklist_item,
    create_checklist_section, delete_checklist, detach_checklist_from_property, get_checklist,
    instantiate_checklist_tree, list_checklist_items, list_checklist_sections, list_checklists,
    list_property_checklists, update_checklist,
};

pub fn checklist_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/checklists", post(create_checklist))
        .route("/api/v1/checklists", get(list_checklists))
        .route("/api/v1/checklists/{id}", get(get_checklist))
        .route("/api/v1/checklists/{id}", put(update_checklist))
        .route("/api/v1/checklists/{id}", delete(delete_checklist))
        .route("/api/v1/checklists/{checklist_id}/sections", post(create_checklist_section))
        .route("/api/v1/checklists/{checklist_id}/sections", get(list_checklist_sections))
        .route("/api/v1/checklists/sections/{section_id}/items", post(create_checklist_item))
        .route("/api/v1/checklists/sections/{section_id}/items", get(list_checklist_items))
}

pub fn property_checklist_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/{property_id}/checklists/instantiate",
            post(instantiate_checklist_tree),
        )
        .route("/{property_id}/checklists", post(attach_checklist_to_property))
        .route("/{property_id}/checklists", get(list_property_checklists))
        .route("/{property_id}/checklists/{checklist_id}", delete(detach_checklist_from_property))
}
