// src/presentation/extractors.rs
// UPDATED: AgencyContext gains an optional `settings` field populated from
// the `Arc<AgencySettings>` that `resolve_agency_context` inserts into
// extensions (after the customisation layer is attached).
//
// Handlers that need settings can do:
//   let settings = ctx.settings.as_ref().map(|s| s.as_ref());
//
// Or read specific values:
//   let quiet_start = ctx.settings.as_ref()
//       .map(|s| s.communication.quiet_start_hour)
//       .unwrap_or(7);

use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::{
    domain::agency::{AgencySettings, ResolvedAgency},
    infrastructure::db::pool::AgencyPool,
};

#[derive(Clone)]
pub struct AgencyContext {
    pub agency: ResolvedAgency,
    pub pool: PgPool,
    /// Populated when the customisation layer is active (i.e. in all
    /// production code).  `None` only in tests that skip `with_customisation`.
    pub settings: Option<Arc<AgencySettings>>,
}

impl<S> FromRequestParts<S> for AgencyContext
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Resolve agency
        let agency = parts
            .extensions
            .get::<ResolvedAgency>()
            .cloned()
            .ok_or_else(|| {
                tracing::error!("AgencyContext: missing ResolvedAgency extension");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error: missing agency context",
                )
                    .into_response()
            })?;

        // 2. Resolve tenant pool
        let pool = parts
            .extensions
            .get::<AgencyPool>()
            .map(|tp| tp.0.clone())
            .ok_or_else(|| {
                tracing::error!("AgencyContext: missing TenantPool extension");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error: missing agency pool",
                )
                    .into_response()
            })?;

        // 3. Resolve AgencySettings — optional, never fails extraction
        let settings = parts.extensions.get::<Arc<AgencySettings>>().cloned();

        Ok(AgencyContext {
            agency,
            pool,
            settings,
        })
    }
}
