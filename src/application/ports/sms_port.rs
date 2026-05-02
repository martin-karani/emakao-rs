use async_trait::async_trait;

use crate::application::errors::AppError;

#[async_trait]
pub trait SmsPort: Send + Sync + 'static {
    async fn send(&self, to: &str, message: &str) -> Result<(), AppError>;
}