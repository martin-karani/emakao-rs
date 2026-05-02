use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{config::Region, Client};

use crate::application::{errors::AppError, ports::storage_port::StoragePort};

pub struct S3Storage {
    client: Client,
    bucket: String,
    endpoint_url: Option<String>,
}

impl S3Storage {
    pub async fn new(
        _access_key_id: &str,
        _secret_access_key: &str,
        region: &str,
        bucket: String,
        endpoint_url: Option<String>,
    ) -> Self {
        // Use Region::new() with an owned String so no &str lifetime escapes
        // into the async config builder.
        let region = Region::new(region.to_owned());

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await;

        let mut s3_config = aws_sdk_s3::config::Builder::from(&config);
        if let Some(ref url) = endpoint_url {
            s3_config = s3_config
                .endpoint_url(url)
                .force_path_style(true);
        }

        Self {
            client: Client::from_conf(s3_config.build()),
            bucket,
            endpoint_url,
        }
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
            .body(data.into())
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        Ok(format!(
            "{}/{}/{}",
            self.endpoint_url.as_deref().unwrap_or(""),
            self.bucket,
            key
        ))
    }

    async fn get_url(&self, key: &str) -> Result<String, AppError> {
        Ok(format!(
            "{}/{}/{}",
            self.endpoint_url.as_deref().unwrap_or(""),
            self.bucket,
            key
        ))
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?;
        Ok(())
    }
}