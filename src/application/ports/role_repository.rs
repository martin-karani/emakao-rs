use crate::application::errors::AppError;
use crate::domain::role::CustomRole;
use async_trait::async_trait;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug)]
pub struct CreateRoleCommand {
    pub agency_id: Uuid,
    pub name: String,
    pub permissions: HashSet<String>,
}

#[derive(Debug)]
pub struct UpdateRoleCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub permissions: HashSet<String>,
}

#[async_trait]
pub trait RoleRepository: Send + Sync + 'static {
    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<CustomRole>, AppError>;
    async fn find_by_name(
        &self,
        agency_id: Uuid,
        name: &str,
    ) -> Result<Option<CustomRole>, AppError>;
    async fn list_for_agency(&self, agency_id: Uuid) -> Result<Vec<CustomRole>, AppError>;
    async fn create(&self, cmd: CreateRoleCommand) -> Result<CustomRole, AppError>;
    async fn update(&self, cmd: UpdateRoleCommand) -> Result<CustomRole, AppError>;
    async fn delete(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError>;
    async fn seed_system_roles(&self, agency_id: Uuid) -> Result<(), AppError>;
}
