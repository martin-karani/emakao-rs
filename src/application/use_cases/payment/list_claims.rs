use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::payment_repository::PaymentRepository},
    domain::payment::{ClaimStatus, PaymentClaim},
};

pub struct ListClaimsUseCase {
    pub repo: Arc<dyn PaymentRepository>,
}

impl ListClaimsUseCase {
    pub fn new(repo: Arc<dyn PaymentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        status: Option<ClaimStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PaymentClaim>, AppError> {
        // Uses: PaymentRepository::find_all
        self.repo.find_all(agency_id, property_id, status, limit, offset).await
    }
}