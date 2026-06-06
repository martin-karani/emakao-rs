use std::{collections::HashSet, sync::Arc};
use uuid::Uuid;
use crate::{
    application::{
        errors::AppError,
        ports::role_repository::{CreateRoleCommand, RoleRepository},
    },
    domain::role::CustomRole,
};

pub struct CreateRoleInput {
    pub agency_id: Uuid,
    pub name: String,
    pub permissions: HashSet<String>,
}

pub struct CreateRoleUseCase {
    repo: Arc<dyn RoleRepository>,
}

impl CreateRoleUseCase {
    pub fn new(repo: Arc<dyn RoleRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateRoleInput) -> Result<CustomRole, AppError> {
        if input.name.trim().is_empty() {
            return Err(AppError::Validation("Role name cannot be empty".into()));
        }

        self.repo
            .create(CreateRoleCommand {
                agency_id: input.agency_id,
                name: input.name,
                permissions: input.permissions,
            })
            .await
    }
}
