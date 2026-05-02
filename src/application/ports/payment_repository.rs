use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::payment::{ClaimStatus, CreatePaymentClaimCommand, PaymentClaim},
};

#[async_trait]
pub trait PaymentRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        status: Option<ClaimStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PaymentClaim>, AppError>;

    async fn find_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<PaymentClaim>, AppError>;

    async fn create(
        &self,
        cmd: CreatePaymentClaimCommand,
    ) -> Result<PaymentClaim, AppError>;

    async fn update_status(
        &self,
        id: Uuid,
        status: ClaimStatus,
        reviewed_by: Uuid,
        review_notes: Option<String>,
        rejection_reason: Option<String>,
    ) -> Result<PaymentClaim, AppError>;
}