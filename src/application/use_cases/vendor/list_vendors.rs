use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::vendor_repository::VendorRepository},
    domain::vendor::Vendor,
};

pub struct ListVendorsUseCase {
    pub repo: Arc<dyn VendorRepository>,
}

impl ListVendorsUseCase {
    pub fn new(repo: Arc<dyn VendorRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Vendor>, AppError> {
        self.repo.find_all(agency_id, limit, offset).await
    }
}
