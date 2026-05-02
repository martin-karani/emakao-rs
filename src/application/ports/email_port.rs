
use async_trait::async_trait;

use crate::application::errors::AppError;

#[async_trait]
pub trait EmailPort: Send + Sync + 'static {
    async fn send(&self, to: &str, subject: &str, html_body: &str) -> Result<(), AppError>;
}