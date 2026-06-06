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
    primitives::ByteStream,
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

        let client = Client::from_conf(s3_cfg.build());

        // In dev mode (endpoint_url present), try to create the bucket.
        // This makes Minio setup zero-config for new developers.
        if endpoint_url.is_some() {
            let _ = client.create_bucket().bucket(&bucket).send().await;
        }

        Self { client, bucket }
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

#[async_trait]
impl StoragePort for S3Storage {
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, AppError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("S3 upload failed: {e}")))?;

        Ok(key.to_string())
    }

    async fn get_url(&self, key: &str) -> Result<String, AppError> {
        // Default to 15-minute presigned URLs for all storage operations.
        self.presigned_url(key, 900).await
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("S3 delete failed: {e}")))?;

        Ok(())
    }
}
