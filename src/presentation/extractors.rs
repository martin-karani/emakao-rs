use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::{domain::agency::ResolvedAgency, infrastructure::db::pool::TenantPool};

#[derive(Clone)]
pub struct TenantContext {
    pub agency: ResolvedAgency,
    pub pool: PgPool,
}

impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Resolve agency from extensions
        let agency = parts
            .extensions
            .get::<ResolvedAgency>()
            .cloned()
            .ok_or_else(|| {
                tracing::error!("TenantContext: missing ResolvedAgency extension");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error: missing agency context",
                )
                    .into_response()
            })?;

        // 2. Resolve tenant pool from extensions
        let pool = parts
            .extensions
            .get::<TenantPool>()
            .map(|tp| tp.0.clone())
            .ok_or_else(|| {
                tracing::error!("TenantContext: missing TenantPool extension");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error: missing tenant pool",
                )
                    .into_response()
            })?;

        Ok(TenantContext { agency, pool })
    }
}
