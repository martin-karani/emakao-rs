use crate::application::errors::AppError;
use async_trait::async_trait;

/// Port for the OpenFGA authorisation service.
///
/// Every method is `store_id`-scoped so a single adapter instance can serve
/// the entire multi-agency fleet — one store per agency.
///
/// ## Model lifecycle
/// 1. `create_store`     — called once at agency provisioning time
/// 2. `write_auth_model` — called immediately after to seed the default model
/// 3. `write_tuple` / `delete_tuple` — runtime permission management
/// 4. `check`            — fast path used inside protected handlers
///
/// An agency can later call `write_auth_model` again to publish a new model
/// version; OpenFGA keeps old versions immutable, so tuples automatically
/// resolve against the latest model in the store.
#[async_trait]
pub trait OpenFgaPort: Send + Sync + 'static {
    // ── Store lifecycle ────────────────────────────────────────────────────

    /// Create a new OpenFGA store and return its opaque `store_id`.
    ///
    /// Implementations SHOULD be idempotent (cache by name) so that accidental
    /// double-calls during provisioning do not create duplicate stores.
    async fn create_store(&self, name: &str) -> Result<String, AppError>;

    // ── Authorization model management ─────────────────────────────────────

    /// Write a new authorization model version into `store_id`.
    ///
    /// `model` must be a valid OpenFGA JSON model object, e.g.:
    ///
    /// ```json
    /// {
    ///   "schema_version": "1.1",
    ///   "type_definitions": [ ... ]
    /// }
    /// ```
    ///
    /// Returns the newly created `authorization_model_id`.  Store this if you
    /// want deterministic pinning; otherwise OpenFGA uses the latest version.
    async fn write_auth_model(
        &self,
        store_id: &str,
        model: &serde_json::Value,
    ) -> Result<String, AppError>;

    // ── Tuple management ───────────────────────────────────────────────────

    /// Grant `relation` between `user` and `object` in `store_id`.
    ///
    /// Tuple format: `user = "user:<uuid>"`, `object = "property:<uuid>"`.
    async fn write_tuple(
        &self,
        store_id: &str,
        user: &str,
        relation: &str,
        object: &str,
    ) -> Result<(), AppError>;

    /// Revoke a previously granted tuple.
    async fn delete_tuple(
        &self,
        store_id: &str,
        user: &str,
        relation: &str,
        object: &str,
    ) -> Result<(), AppError>;

    // ── Permission checks ──────────────────────────────────────────────────

    /// Returns `true` when `user` has `relation` on `object` in `store_id`.
    ///
    /// Pass `None` for `authorization_model_id` to use the latest model
    /// version automatically.
    async fn check(
        &self,
        store_id: &str,
        user: &str,
        relation: &str,
        object: &str,
        authorization_model_id: Option<&str>,
    ) -> Result<bool, AppError>;
}
