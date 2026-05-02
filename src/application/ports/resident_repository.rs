use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::resident::Resident,
};

pub struct CreateResidentCommand {
    pub agency_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub national_id: Option<String>,
}

#[async_trait]
pub trait ResidentRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Resident>, AppError>;

    async fn find_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Resident>, AppError>;

    async fn find_by_email(
        &self,
        agency_id: Uuid,
        email: &str,
    ) -> Result<Option<Resident>, AppError>;

    async fn create(
        &self,
        cmd: CreateResidentCommand,
    ) -> Result<Resident, AppError>;
}