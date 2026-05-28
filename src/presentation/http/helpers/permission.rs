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
/// `Ok(())` only when **all** pass.  Short-circuits on the first failure.
///
/// ISSUE 20 FIX: previously used `join_all` which ran all FGA checks even
/// after one had already failed, wasting RPC calls. Now uses `try_join_all`
/// which cancels remaining futures on first error.
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

    // Run concurrently; cancel on first failure.
    futures::future::try_join_all(futs).await?;
    Ok(())
}

// ── Guarded helper ────────────────────────────────────────────────────────────

/// Runs the OpenFGA permission check for the agency's FGA store.
///
/// ISSUE 6 FIX: previously this returned `Ok(())` when `fga_store_id` was
/// `None`, silently bypassing all fine-grained authorization for agencies
/// that lost or never had their FGA store provisioned.  Now it hard-errors,
/// ensuring that a misconfigured agency cannot inadvertently grant full access.
///
/// If you need a lenient mode for local development, set a real (local)
/// OpenFGA store rather than leaving the column NULL.
pub async fn check_permission(
    state: &AppState,
    ctx: &AgencyContext,
    user: &AuthenticatedUser,
    relation: &str,
    object: &str,
) -> Result<(), AppError> {
    let store_id = ctx
        .agency
        .fga_store_id
        .as_deref()
        .ok_or_else(|| {
            AppError::InternalServer(
                "Agency has no OpenFGA store configured — contact platform admin".into(),
            )
        })?;

    require_permission(
        state.openfga.as_ref(),
        store_id,
        user.user_id,
        relation,
        object,
    )
    .await
}
