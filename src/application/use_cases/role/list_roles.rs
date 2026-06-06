use std::sync::Arc;
use uuid::Uuid;
use crate::{
    application::{errors::AppError, ports::role_repository::RoleRepository},
    domain::role::CustomRole,
};

pub struct ListRolesUseCase {
    repo: Arc<dyn RoleRepository>,
}

impl ListRolesUseCase {
    pub fn new(repo: Arc<dyn RoleRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid) -> Result<Vec<CustomRole>, AppError> {
        self.repo.list_for_agency(agency_id).await
    }
}
