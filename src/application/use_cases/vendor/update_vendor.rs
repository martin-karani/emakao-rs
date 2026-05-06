use std::sync::Arc;

use crate::{
    application::{errors::AppError, ports::vendor_repository::VendorRepository},
    domain::vendor::{UpdateVendorCommand, Vendor},
};

pub struct UpdateVendorUseCase {
    pub repo: Arc<dyn VendorRepository>,
}

impl UpdateVendorUseCase {
    pub fn new(repo: Arc<dyn VendorRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, cmd: UpdateVendorCommand) -> Result<Vendor, AppError> {
        self.repo.update(cmd).await
    }
}
