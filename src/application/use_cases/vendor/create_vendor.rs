use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::vendor_repository::VendorRepository},
    domain::vendor::{CreateVendorCommand, Vendor},
};

pub struct CreateVendorUseCase {
    pub repo: Arc<dyn VendorRepository>,
}

pub struct CreateVendorInput {
    pub agency_id: Uuid,
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub speciality: Option<String>,
    pub notes: Option<String>,
}

impl CreateVendorUseCase {
    pub fn new(repo: Arc<dyn VendorRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateVendorInput) -> Result<Vendor, AppError> {
        if input.name.trim().is_empty() {
            return Err(AppError::Validation("Vendor name must not be empty".into()));
        }
        let vendor = self
            .repo
            .create(CreateVendorCommand {
                agency_id: input.agency_id,
                user_id: None, // directory-only
                name: input.name,
                contact_name: input.contact_name,
                email: input.email,
                phone: input.phone,
                speciality: input.speciality,
                notes: input.notes,
            })
            .await?;
        tracing::info!(vendor_id = %vendor.id, "vendor created without portal access");
        Ok(vendor)
    }
}
