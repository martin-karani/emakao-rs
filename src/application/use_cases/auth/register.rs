use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            agency_repository::AgencyRepository, auth_port::AuthPort,
            auth_repository::{AuthRepository, CreateMembershipCommand, CreateUserCommand},
        },
    },
    domain::auth::StoredUser,
};

pub struct RegisterUseCase {
    pub repo: Arc<dyn AuthRepository>,
    pub agency_repo: Arc<dyn AgencyRepository>,
    pub auth: Arc<dyn AuthPort>,
}

pub struct RegisterInput {
    pub agency_id: Uuid,
    pub email: String,
    pub password: String,
    pub role: String,
}

impl RegisterUseCase {
    pub fn new(repo: Arc<dyn AuthRepository>, agency_repo: Arc<dyn AgencyRepository>, auth: Arc<dyn AuthPort>) -> Self {
        Self { repo, agency_repo, auth }
    }

    pub async fn execute(&self, input: RegisterInput) -> Result<StoredUser, AppError> {
        if input.password.len() < 8 {
            return Err(AppError::Validation(
                "password must be at least 8 characters".into(),
            ));
        }

        // Uses: AgencyRepository::contact_exists_for_agency — guard duplicate
        if self
            .agency_repo
            .contact_exists_for_agency(input.agency_id, &input.email, "email", &input.role)
            .await?
        {
            return Err(AppError::Validation(format!(
                "email '{}' is already registered for this role",
                input.email
            )));
        }

        // Uses: AuthPort::hash_password
        let password_hash = self.auth.hash_password(&input.password).await?;
        let user_id = Uuid::new_v4();

        // Uses: AuthRepository::create_user
        let user = self
            .repo
            .create_user(CreateUserCommand {
                id: user_id,
                email: Some(input.email.clone()),
                phone: None,
                password_hash,
                is_active: true,
                must_change_password: false,
            })
            .await?;

        // 2. Create membership for the agency
        self.repo
            .create_membership(CreateMembershipCommand {
                user_id: user.id,
                agency_id: input.agency_id,
                role: input.role,
            })
            .await?;

        tracing::info!(user_id = %user.id, email = %user.email.as_deref().unwrap_or(""), "user registered");
        Ok(user)
    }
}
