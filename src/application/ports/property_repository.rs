use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::{
        property::{CreatePropertyCommand, Property, UpdatePropertyCommand},
        property_summary::PropertySummary,
    },
};

pub struct PropertyFilter {
    pub agency_id: Uuid,
    pub property_type: Option<String>,
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

pub struct PropertySummaryQuery {
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub expiring_lease_days: i64,
}

pub struct PropertyTeamMember {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: String,
    pub status: String,
}

#[async_trait]
pub trait PropertyRepository: Send + Sync + 'static {
    async fn get_team(&self, agency_id: Uuid, property_id: Uuid) -> Result<Vec<PropertyTeamMember>, AppError>;
    async fn get_summary(&self, query: PropertySummaryQuery) -> Result<PropertySummary, AppError>;
    async fn find_all(&self, filter: PropertyFilter) -> Result<Vec<Property>, AppError>;
    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Property>, AppError>;
    async fn find_by_slug(&self, agency_id: Uuid, slug: &str) -> Result<Option<Property>, AppError>;
    async fn find_by_name(&self, agency_id: Uuid, name: &str) -> Result<Option<Property>, AppError>;
    async fn list_slugs_for_agency(&self, agency_id: Uuid) -> Result<Vec<String>, AppError>;
    async fn create(&self, cmd: CreatePropertyCommand) -> Result<Property, AppError>;
    async fn update(&self, cmd: UpdatePropertyCommand) -> Result<Property, AppError>;
    async fn delete(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError>;
    async fn count_for_agency(&self, agency_id: Uuid) -> Result<i64, AppError>;
}
