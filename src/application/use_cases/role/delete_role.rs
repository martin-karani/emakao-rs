use std::sync::Arc;
use uuid::Uuid;
use crate::application::{errors::AppError, ports::role_repository::RoleRepository};

pub struct DeleteRoleUseCase {
    repo: Arc<dyn RoleRepository>,
}

impl DeleteRoleUseCase {
    pub fn new(repo: Arc<dyn RoleRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError> {
        self.repo.delete(agency_id, id).await
    }
}
