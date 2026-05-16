// src/application/ports/notification_port.rs

use crate::application::errors::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait NotificationPort: Send + Sync {
    async fn send_password_reset(&self, email: &str, token: &str) -> Result<(), AppError>;
}
