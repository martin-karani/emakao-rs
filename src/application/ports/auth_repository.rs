use async_trait::async_trait;
use uuid::Uuid;

use crate::{application::errors::AppError, domain::auth::StoredUser};

pub struct CreateUserCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: String,
}

#[async_trait]
pub trait AuthRepository: Send + Sync + 'static {
    async fn find_by_email(
        &self,
        agency_id: Uuid,
        email: &str,
    ) -> Result<Option<StoredUser>, AppError>;

    async fn create(&self, cmd: CreateUserCommand) -> Result<StoredUser, AppError>;

    async fn email_exists(&self, agency_id: Uuid, email: &str) -> Result<bool, AppError>;
}