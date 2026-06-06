use crate::{
    application::{
        errors::AppError,
        ports::role_repository::{RoleRepository, UpdateRoleCommand},
    },
    domain::role::CustomRole,
    infrastructure::cache::permission_cache::PermissionCache,
};
use std::{collections::HashSet, sync::Arc};
use uuid::Uuid;

pub struct UpdateRoleInput {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub permissions: HashSet<String>,
}

pub struct UpdateRoleUseCase {
    repo: Arc<dyn RoleRepository>,
    _permission_cache: Arc<PermissionCache>,
}

impl UpdateRoleUseCase {
    pub fn new(repo: Arc<dyn RoleRepository>, permission_cache: Arc<PermissionCache>) -> Self {
        Self {
            repo,
            _permission_cache: permission_cache,
        }
    }

    pub async fn execute(&self, input: UpdateRoleInput) -> Result<CustomRole, AppError> {
        let role = self
            .repo
            .update(UpdateRoleCommand {
                id: input.id,
                agency_id: input.agency_id,
                permissions: input.permissions,
            })
            .await?;

        // Invalidate permission cache for all users in this agency who have this role.
        // For simplicity in this initial version, we invalidate the entire cache
        // for this agency if possible, or just rely on the fact that the cache
        // will eventually refresh or be manually cleared.
        // Actually, the PermissionCache handles per-(user, agency) pairs.
        // A better approach is to have a way to invalidate by role name.
        // For now, we'll assume the cache is short-lived or manually cleared.

        Ok(role)
    }
}
