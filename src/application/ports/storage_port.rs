use async_trait::async_trait;

use crate::application::errors::AppError;

#[async_trait]
pub trait StoragePort: Send + Sync + 'static {
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, AppError>;
    async fn get_url(&self, key: &str) -> Result<String, AppError>;
    async fn delete(&self, key: &str) -> Result<(), AppError>;
}
