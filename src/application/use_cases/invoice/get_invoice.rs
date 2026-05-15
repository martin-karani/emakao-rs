use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::invoice_repository::InvoiceRepository},
    domain::invoice::Invoice,
};

pub struct GetInvoiceUseCase {
    pub repo: Arc<dyn InvoiceRepository>,
}

impl GetInvoiceUseCase {
    pub fn new(repo: Arc<dyn InvoiceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<Invoice, AppError> {
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("invoice {id}")))
    }
}
