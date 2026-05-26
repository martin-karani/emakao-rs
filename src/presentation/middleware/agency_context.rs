use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::{
    domain::{agency::ResolvedAgency, auth::AuthenticatedUser},
    infrastructure::db::pool::AgencyPool,
    presentation::app_state::AppState,
};

pub async fn resolve_agency_context(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // ── 1. Require AuthenticatedUser (set by require_auth middleware) ──────────
    let user = match req.extensions().get::<AuthenticatedUser>().cloned() {
        Some(u) => u,
        None => {
            tracing::error!(
                "resolve_agency_context: AuthenticatedUser missing — require_auth must run first"
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let platform = state.infra.tenant_pools.platform();

    // ── 2. Load agency row from platform DB (UNCHANGED) ───────────────────────
    let row = sqlx::query(
        r#"
        SELECT id, name, slug, schema_name, fga_store_id
        FROM   agencies
        WHERE  id     = $1
          AND  status = 'active'
        LIMIT 1
        "#,
    )
    .bind(user.agency_id)
    .fetch_optional(platform)
    .await;

    let resolved = match row {
        Ok(Some(r)) => {
            use sqlx::Row;
            ResolvedAgency {
                id: r.get("id"),
                name: r.get("name"),
                slug: r.get("slug"),
                schema_name: r.get("schema_name"),
                fga_store_id: r.get("fga_store_id"),
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error":   "AGENCY_NOT_FOUND",
                    "message": "The agency associated with this token no longer exists.",
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "resolve_agency_context: platform DB query failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // ── 3. Build agency-schema pool (UNCHANGED) ───────────────────────────────
    let tenant_pool = match state
        .infra
        .tenant_pools
        .for_tenant(&resolved.schema_name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(
                error  = %e,
                schema = %resolved.schema_name,
                "resolve_agency_context: failed to build agency pool"
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // ── 4. NEW: load AgencySettings from customisation cache ──────────────────
    if let Some(custom) = &state.custom {
        match custom.settings.get_or_load(resolved.id, platform).await {
            Ok(settings) => {
                req.extensions_mut().insert(settings); // Arc<AgencySettings>
            }
            Err(e) => {
                // Non-fatal: log and continue.  Handlers that require settings
                // will fail with a clear error when they try to extract the
                // extension.
                tracing::warn!(
                    agency = %resolved.id,
                    error  = %e,
                    "resolve_agency_context: could not load AgencySettings — continuing without"
                );
            }
        }
    }

    // ── 5. Insert extensions (unchanged names) ────────────────────────────────
    req.extensions_mut().insert(resolved);
    req.extensions_mut().insert(AgencyPool(tenant_pool));

    next.run(req).await
}
