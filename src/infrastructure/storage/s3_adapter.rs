// src/infrastructure/storage/s3_adapter.rs
//
// AWS S3 / MinIO storage adapter.
//
// ## Lifecycle
//
// Build ONE `S3Storage` in `AppState::build` and place it behind
// `Arc<dyn StoragePort>`.  Every handler that needs storage receives it via
// `state.storage.clone()`.  Never construct a new `S3Storage` inside a handler
// or use-case — the SDK config load is async and expensive.
//
// ## Presigned URLs
//
// `get_url` returns a 15-minute presigned GET URL.  If your bucket is
// public-read, override this behaviour by replacing the impl with a plain
// `format!("https://{bucket}.s3.{region}.amazonaws.com/{key}")` call.

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    config::{Credentials, Region},
    presigning::PresigningConfig,
    Client,
};
use std::time::Duration;

use crate::application::{errors::AppError, ports::storage_port::StoragePort};

pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    /// Build a fully-configured S3 client.
    ///
    /// Called **once** from `AppState::build`.  The resulting instance is
    /// shared across the process lifetime via `Arc<dyn StoragePort>`.
    pub async fn new(
        access_key_id: &str,
        secret_access_key: &str,
        region: &str,
        bucket: String,
        endpoint_url: Option<&str>,
    ) -> Self {
        let region = Region::new(region.to_owned());

        let creds = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "emakao-static",
        );

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .credentials_provider(creds)
            .load()
            .await;

        let mut s3_cfg = aws_sdk_s3::config::Builder::from(&config);
        if let Some(url) = endpoint_url {
            s3_cfg = s3_cfg.endpoint_url(url).force_path_style(true);
        }

        Self {
            client: Client::from_conf(s3_cfg.build()),
            bucket,
        }
    }

    /// Generate a pre-signed GET URL valid for `ttl_seconds` seconds.
    ///
    /// Use this when you want clients to download directly from S3 without
    /// proxying through the API server.
    pub async fn presigned_url(&self, key: &str, ttl_seconds: u64) -> Result<String, AppError> {
        let expires = PresigningConfig::expires_in(Duration::from_secs(ttl_seconds))
            .map_err(|e| AppError::ExternalService(format!("presign config: {e}")))?;

        let url = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(expires)
            .await
            .map_err(|e| AppError::ExternalService(format!("presign failed: {e}")))?
            .uri()
            .to_string();

        Ok(url)
    }
}
