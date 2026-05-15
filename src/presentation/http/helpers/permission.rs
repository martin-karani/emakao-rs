use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::openfga_port::OpenFgaPort},
    domain::auth::AuthenticatedUser,
    presentation::{app_state::AppState, extractors::AgencyContext},
};

// ── Core helper ───────────────────────────────────────────────────────────────

/// ## Example
///
/// ```rust
/// require_permission(
///     &state.openfga,
///     &store_id,
///     claims.sub,
///     "can_edit",
///     &format!("property:{property_id}"),
/// ).await?;
/// ```
pub async fn require_permission(
    openfga: &dyn OpenFgaPort,
    store_id: &str,
    user_id: Uuid,
    relation: &str,
    object: &str,
) -> Result<(), AppError> {
    let user = format!("user:{user_id}");

    if !openfga
        .check(store_id, &user, relation, object, None)
        .await?
    {
        return Err(AppError::Forbidden(format!(
            "user {user_id} does not have '{relation}' on '{object}'"
        )));
    }
    Ok(())
}

// ── Batch helper ──────────────────────────────────────────────────────────────

/// Check multiple `(relation, object)` pairs concurrently and return
/// `Ok(())` only when **all** pass.  First failure short-circuits.
///
/// Useful for multi-step handlers that require several permissions.
pub async fn require_permissions(
    openfga: Arc<dyn OpenFgaPort>,
    store_id: String,
    user_id: Uuid,
    checks: Vec<(&'static str, String)>, // (relation, object)
) -> Result<(), AppError> {
    let user = format!("user:{user_id}");

    let futs = checks.into_iter().map(|(relation, object)| {
        let openfga = Arc::clone(&openfga);
        let store_id = store_id.clone();
        let user = user.clone();
        async move {
            if !openfga
                .check(&store_id, &user, relation, &object, None)
                .await?
            {
                return Err(AppError::Forbidden(format!(
                    "user {user_id} does not have '{relation}' on '{object}'"
                )));
            }
            Ok(())
        }
    });

    // Run all checks concurrently; collect errors.
    let results = futures::future::join_all(futs).await;
    for r in results {
        r?;
    }
    Ok(())
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Runs the OpenFGA permission check only when an FGA store is configured.
/// When the agency has no store (e.g. during local dev) the check is skipped.
pub async fn check_permission(
    state: &AppState,
    ctx: &AgencyContext,
    user: &AuthenticatedUser,
    relation: &str,
    object: &str,
) -> Result<(), AppError> {
    if let Some(store_id) = ctx.agency.fga_store_id.as_deref() {
        require_permission(
            state.openfga.as_ref(),
            store_id,
            user.user_id,
            relation,
            object,
        )
        .await?;
    }
    Ok(())
}
