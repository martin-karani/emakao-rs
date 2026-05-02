// src/infrastructure/openfga/openfga_adapter.rs

use async_trait::async_trait;
use dashmap::DashMap;
use reqwest::Client;
use std::sync::Arc;

use crate::application::{errors::AppError, ports::openfga_port::OpenFgaPort};

/// REST-based OpenFGA adapter.
///
/// Communicates with the OpenFGA HTTP API (default: `http://localhost:8080`).
/// A single adapter instance handles every agency — calls are always
/// scoped by `store_id`.
///
/// ## Thread safety
/// `OpenFgaAdapter` is `Clone + Send + Sync`.  The `store_cache` is backed by
/// a `DashMap` so concurrent agency-provisioning calls are safe.
pub struct OpenFgaAdapter {
    base_url: String,
    client: Client,
    /// Name → store_id cache.  Prevents duplicate store creation on retry.
    store_cache: Arc<DashMap<String, String>>,
}

impl OpenFgaAdapter {
    /// `base_url` — e.g. `http://localhost:8080` (no trailing slash).
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
            store_cache: Arc::new(DashMap::new()),
        }
    }

    // ── URL builders ───────────────────────────────────────────────────────

    fn stores_url(&self) -> String {
        format!("{}/stores", self.base_url)
    }

    fn auth_models_url(&self, store_id: &str) -> String {
        format!("{}/stores/{}/authorization-models", self.base_url, store_id)
    }

    fn tuples_url(&self, store_id: &str) -> String {
        format!("{}/stores/{}/write", self.base_url, store_id)
    }

    fn check_url(&self, store_id: &str) -> String {
        format!("{}/stores/{}/check", self.base_url, store_id)
    }
}

#[async_trait]
impl OpenFgaPort for OpenFgaAdapter {
    // ── Store lifecycle ────────────────────────────────────────────────────

    async fn create_store(&self, name: &str) -> Result<String, AppError> {
        // Return cached id if we already created it in this process.
        if let Some(id) = self.store_cache.get(name) {
            tracing::debug!(store_name = name, "create_store: returning cached id");
            return Ok(id.clone());
        }

        let body = serde_json::json!({ "name": name });

        let resp = self
            .client
            .post(self.stores_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenFGA create_store: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenFGA create_store HTTP {status}: {text}"
            )));
        }

        #[derive(serde::Deserialize)]
        struct StoreResp {
            id: String,
        }

        let store: StoreResp = resp
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenFGA create_store parse: {e}")))?;

        self.store_cache.insert(name.to_owned(), store.id.clone());
        tracing::info!(store_id = %store.id, store_name = name, "OpenFGA store created");

        Ok(store.id)
    }

    // ── Authorization model management ─────────────────────────────────────

    async fn write_auth_model(
        &self,
        store_id: &str,
        model: &serde_json::Value,
    ) -> Result<String, AppError> {
        // OpenFGA expects the model fields at the top level of the body,
        // so we POST the JSON value directly.
        let resp = self
            .client
            .post(self.auth_models_url(store_id))
            .json(model)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenFGA write_auth_model: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenFGA write_auth_model HTTP {status}: {text}"
            )));
        }

        #[derive(serde::Deserialize)]
        struct AuthModelResp {
            authorization_model_id: String,
        }

        let parsed: AuthModelResp = resp.json().await.map_err(|e| {
            AppError::ExternalService(format!("OpenFGA write_auth_model parse: {e}"))
        })?;

        tracing::info!(
            store_id,
            model_id = %parsed.authorization_model_id,
            "OpenFGA authorization model written"
        );

        Ok(parsed.authorization_model_id)
    }

    // ── Tuple management ───────────────────────────────────────────────────

    async fn write_tuple(
        &self,
        store_id: &str,
        user: &str,
        relation: &str,
        object: &str,
    ) -> Result<(), AppError> {
        let body = serde_json::json!({
            "writes": {
                "tuple_keys": [{
                    "user":     user,
                    "relation": relation,
                    "object":   object
                }]
            }
        });

        let resp = self
            .client
            .post(self.tuples_url(store_id))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenFGA write_tuple: {e}")))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenFGA write_tuple error: {text}"
            )));
        }
        Ok(())
    }

    async fn delete_tuple(
        &self,
        store_id: &str,
        user: &str,
        relation: &str,
        object: &str,
    ) -> Result<(), AppError> {
        let body = serde_json::json!({
            "deletes": {
                "tuple_keys": [{
                    "user":     user,
                    "relation": relation,
                    "object":   object
                }]
            }
        });

        let resp = self
            .client
            .post(self.tuples_url(store_id))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenFGA delete_tuple: {e}")))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenFGA delete_tuple error: {text}"
            )));
        }
        Ok(())
    }

    // ── Permission checks ──────────────────────────────────────────────────

    async fn check(
        &self,
        store_id: &str,
        user: &str,
        relation: &str,
        object: &str,
        authorization_model_id: Option<&str>,
    ) -> Result<bool, AppError> {
        let mut body = serde_json::json!({
            "tuple_key": {
                "user":     user,
                "relation": relation,
                "object":   object
            }
        });

        // Pin to a specific model version when provided.
        // If None, OpenFGA resolves against the latest model in the store.
        if let Some(model_id) = authorization_model_id {
            body["authorization_model_id"] = serde_json::Value::String(model_id.to_owned());
        }

        let resp: serde_json::Value = self
            .client
            .post(self.check_url(store_id))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenFGA check send: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenFGA check parse: {e}")))?;

        Ok(resp["allowed"].as_bool().unwrap_or(false))
    }
}
