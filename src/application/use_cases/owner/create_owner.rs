use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::owner_repository::OwnerRepository},
    domain::owner::{CreateOwnerCommand, Owner},
};

pub struct CreateOwnerUseCase {
    pub repo: Arc<dyn OwnerRepository>,
}

pub struct CreateOwnerInput {
    pub agency_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub kra_pin: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub mpesa_number: Option<String>,
}

impl CreateOwnerUseCase {
    pub fn new(repo: Arc<dyn OwnerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateOwnerInput) -> Result<Owner, AppError> {
        if input.first_name.trim().is_empty() || input.last_name.trim().is_empty() {
            return Err(AppError::Validation("Owner name must not be empty".into()));
        }
        // Optionally check duplicate by email/phone
        let owner = self
            .repo
            .create(CreateOwnerCommand {
                agency_id: input.agency_id,
                user_id: None,
                first_name: input.first_name,
                last_name: input.last_name,
                email: input.email,
                phone: input.phone,
                company_name: input.company_name,
                kra_pin: input.kra_pin,
                bank_name: input.bank_name,
                bank_account: input.bank_account,
                mpesa_number: input.mpesa_number,
            })
            .await?;
        tracing::info!(owner_id = %owner.id, "owner created without portal access");
        Ok(owner)
    }
}
