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
    // AuthenticatedUser must already be present (require_auth runs first).
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

    let row = sqlx::query!(
        r#"
        SELECT id, name, slug, schema_name AS "schema_name!", fga_store_id
        FROM   agencies
        WHERE  id     = $1
          AND  status = 'active'
        LIMIT 1
        "#,
        user.agency_id
    )
    .fetch_optional(platform)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error":   "AGENCY_NOT_FOUND",
                    "message": "The agency associated with this token no longer exists."
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "resolve_agency_context db error");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let tenant_pool = match state.infra.tenant_pools.for_tenant(&row.schema_name).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, schema = %row.schema_name, "failed to build agency pool");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    req.extensions_mut().insert(ResolvedAgency {
        id: row.id,
        name: row.name,
        slug: row.slug,
        schema_name: row.schema_name,
        fga_store_id: row.fga_store_id,
    });
    req.extensions_mut().insert(AgencyPool(tenant_pool));

    next.run(req).await
}
