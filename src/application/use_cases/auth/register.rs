use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{auth_port::AuthPort, auth_repository::{AuthRepository, CreateUserCommand}},
    },
    domain::auth::StoredUser,
};

pub struct RegisterUseCase {
    pub repo: Arc<dyn AuthRepository>,
    pub auth: Arc<dyn AuthPort>,
}

pub struct RegisterInput {
    pub agency_id: Uuid,
    pub email: String,
    pub password: String,
    pub role: String,
}

impl RegisterUseCase {
    pub fn new(repo: Arc<dyn AuthRepository>, auth: Arc<dyn AuthPort>) -> Self {
        Self { repo, auth }
    }

    pub async fn execute(&self, input: RegisterInput) -> Result<StoredUser, AppError> {
        if input.password.len() < 8 {
            return Err(AppError::Validation("password must be at least 8 characters".into()));
        }

        // Uses: AuthRepository::email_exists — guard duplicate
        if self.repo.email_exists(input.agency_id, &input.email).await? {
            return Err(AppError::Validation(
                format!("email '{}' is already registered", input.email),
            ));
        }

        // Uses: AuthPort::hash_password
        let password_hash = self.auth.hash_password(&input.password).await?;
        let user_id = Uuid::new_v4();

        // Uses: AuthRepository::create
        let user = self
            .repo
            .create(CreateUserCommand {
                id: user_id,
                agency_id: input.agency_id,
                email: input.email.clone(),
                password_hash,
                role: input.role,
            })
            .await?;

        tracing::info!(user_id = %user.id, email = %user.email, "user registered");
        Ok(user)
    }
}