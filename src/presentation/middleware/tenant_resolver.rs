use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::{
    domain::agency::{PortalType, ResolvedAgency},
    infrastructure::db::pool::TenantPool,
    presentation::app_state::AppState,
};

pub async fn tenant_resolver(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let slug = match extract_slug(&req) {
        Some(s) => s,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "MISSING_TENANT",
                    "message": "could not determine agency — provide X-Agency-Slug header or subdomain"
                })),
            )
                .into_response();
        }
    };

    let platform = state.tenant_pools.platform();

    // ── Look up the agency ────────────────────────────────────────────────────
    //
    // We query agencies only — NOT subscriptions.  Subscription enforcement
    // belongs in `subscription_middleware` so that:
    //   • Auth routes work for agencies that have no subscription yet.
    //   • Billing routes work so agencies can subscribe / pay.
    //   • Trialing agencies (status = 'trialing') are not locked out.
    //
    // We still filter on `status = 'active'` so that suspended or deleted
    // agencies are rejected at the door.
    let row = sqlx::query!(
        r#"
        SELECT id, schema_name AS "schema_name!", fga_store_id
        FROM   agencies
        WHERE  slug   = $1
          AND  status = 'active'
        LIMIT  1
        "#,
        slug
    )
    .fetch_optional(platform)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "AGENCY_NOT_FOUND",
                    "message": format!("no active agency found for slug '{slug}'")
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "tenant_resolver db error");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let schema = row.schema_name.clone();

    let tenant_pool = match state.tenant_pools.for_tenant(&schema).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, schema, "failed to build tenant pool");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let resolved = ResolvedAgency {
        id: row.id,
        schema_name: schema.to_string(),
        fga_store_id: row.fga_store_id,
        portal_type: PortalType::Staff,
    };

    req.extensions_mut().insert(resolved);
    req.extensions_mut().insert(TenantPool(tenant_pool));

    next.run(req).await
}

/// Extract the agency slug from `X-Agency-Slug` header first, then fall back
/// to the first subdomain of `Host` (e.g. `acme.emakao.co.ke` → `acme`).
fn extract_slug(req: &Request<Body>) -> Option<String> {
    if let Some(v) = req
        .headers()
        .get("x-agency-slug")
        .and_then(|h| h.to_str().ok())
    {
        if !v.is_empty() {
            return Some(v.to_lowercase());
        }
    }

    // Subdomain fallback: `acme.emakao.co.ke` → `acme`
    let host = req.headers().get("host")?.to_str().ok()?;
    let host = host.split(':').next().unwrap_or(host); // strip port
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 3 {
        return Some(parts[0].to_lowercase());
    }

    None
}
