use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::vendor_repository::VendorRepository},
    domain::vendor::Vendor,
};

pub struct GetVendorUseCase {
    pub repo: Arc<dyn VendorRepository>,
}

impl GetVendorUseCase {
    pub fn new(repo: Arc<dyn VendorRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<Vendor, AppError> {
        // Uses: VendorRepository::find_by_id
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("vendor {id}")))
    }
}