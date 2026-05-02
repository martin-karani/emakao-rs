use std::sync::Arc;
use uuid::Uuid;
use crate::application::{errors::AppError, ports::auth_port::AuthPort};

pub struct ChangePasswordUseCase { pub auth: Arc<dyn AuthPort> }

impl ChangePasswordUseCase {
    pub fn new(auth: Arc<dyn AuthPort>) -> Self { Self { auth } }

    pub async fn execute(
        &self,
        user_id: Uuid,
        stored_hash: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<String, AppError> {
        let _ = user_id;
        let valid = self.auth.verify_password(old_password, stored_hash).await?;
        if !valid { return Err(AppError::Unauthorised); }
        if new_password.len() < 8 {
            return Err(AppError::Validation("password must be at least 8 characters".into()));
        }
        self.auth.hash_password(new_password).await
    }
}
