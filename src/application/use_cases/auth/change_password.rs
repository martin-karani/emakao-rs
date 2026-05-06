use crate::application::{
    errors::AppError,
    ports::{auth_port::AuthPort, auth_repository::AuthRepository},
};
use std::sync::Arc;
use uuid::Uuid;

pub struct ChangePasswordInput {
    pub user_id: Uuid,
    pub old_password: String,
    pub new_password: String,
}

pub struct ChangePasswordUseCase {
    pub repo: Arc<dyn AuthRepository>,
    pub auth: Arc<dyn AuthPort>,
}

impl ChangePasswordUseCase {
    pub fn new(repo: Arc<dyn AuthRepository>, auth: Arc<dyn AuthPort>) -> Self {
        Self { repo, auth }
    }

    pub async fn execute(&self, input: ChangePasswordInput) -> Result<(), AppError> {
        if input.new_password.len() < 8 {
            return Err(AppError::Validation(
                "Password must be at least 8 characters.".into(),
            ));
        }

        // Load the current hash
        let user = self
            .repo
            .find_user_by_id(input.user_id)
            .await?
            .ok_or(AppError::Unauthorised)?;

        if !self
            .auth
            .verify_password(&input.old_password, &user.password_hash)
            .await?
        {
            return Err(AppError::Unauthorised);
        }

        let new_hash = self.auth.hash_password(&input.new_password).await?;
        self.repo.activate_user(input.user_id, &new_hash).await?;

        tracing::info!(user_id = %input.user_id, "password changed");
        Ok(())
    }
}
