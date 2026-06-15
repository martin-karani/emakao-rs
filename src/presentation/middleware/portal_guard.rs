// Rejects any request whose JWT portal does not match the route group it is
// hitting. Applied per route group at router build time, not globally.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::domain::{auth::AuthenticatedUser, enums::PortalType};

// ISSUE 11 FIX: Ordering dependency comment.
// This middleware MUST be layered after `require_auth` so that `AuthenticatedUser`
// is present in the request extensions. In Axum, `.layer(A).layer(B)` executes B then A.
// The router mounts this correctly by layering `portal_guard` BEFORE `require_auth`.
pub async fn portal_guard(required: PortalType, req: Request<Body>, next: Next) -> Response {
    let actual = req
        .extensions()
        .get::<AuthenticatedUser>()
        .map(|u| u.portal);

    match actual {
        Some(p) if p == required => next.run(req).await,
        Some(actual_portal) => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "WRONG_PORTAL",
                "message": format!(
                    "This token is for the '{}' portal; you are accessing the '{}' portal.",
                    actual_portal, required
                )
            })),
        )
            .into_response(),
        None => {
            // Should never happen — require_auth runs first.
            tracing::error!("portal_guard: AuthenticatedUser missing");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn resident_portal_guard(req: Request<Body>, next: Next) -> Response {
    portal_guard(PortalType::Resident, req, next).await
}

pub async fn owner_portal_guard(req: Request<Body>, next: Next) -> Response {
    portal_guard(PortalType::Owner, req, next).await
}

pub async fn vendor_portal_guard(req: Request<Body>, next: Next) -> Response {
    portal_guard(PortalType::Vendor, req, next).await
}

// ISSUE 5 FIX: caretaker portal guard was missing; router could not mount
// caretaker routes without it.
pub async fn caretaker_portal_guard(req: Request<Body>, next: Next) -> Response {
    portal_guard(PortalType::Caretaker, req, next).await
}

// ISSUE 14 FIX: Guard to explicitly reject system admin from staff routes.
pub async fn staff_portal_guard(req: Request<Body>, next: Next) -> Response {
    let actual = req.extensions().get::<AuthenticatedUser>();

    match actual {
        Some(u) if u.portal == PortalType::Staff => {
            next.run(req).await
        }
        Some(_) => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "WRONG_PORTAL",
                "message": "This token is for platform admin or another portal; you cannot access staff operational routes"
            })),
        )
            .into_response(),
        None => {
            tracing::error!("portal_guard: AuthenticatedUser missing");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
