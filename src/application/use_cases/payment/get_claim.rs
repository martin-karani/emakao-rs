use std::sync::Arc;
use uuid::Uuid;
use crate::{
    application::{errors::AppError, ports::payment_repository::PaymentRepository},
    domain::payment::PaymentClaim,
};

pub struct GetClaimUseCase { pub repo: Arc<dyn PaymentRepository> }

impl GetClaimUseCase {
    pub fn new(repo: Arc<dyn PaymentRepository>) -> Self { Self { repo } }
    
    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<PaymentClaim, AppError> {
        self.repo.find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("payment claim {id}")))
    }
}
