use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    infrastructure::cache::subscription_cache::SubscriptionCache,
};

pub struct RecordPaymentUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
}

impl RecordPaymentUseCase {
    /// Called from the M-Pesa callback after a successful STK confirmation.
    pub async fn execute(
        &self,
        agency_id: Uuid,
        amount_kes: i32,
        mpesa_ref: &str,
        mpesa_phone: Option<&str>,
    ) -> Result<Uuid, AppError> {
        let invoice_id = self
            .repo
            .record_payment(agency_id, amount_kes, mpesa_ref, mpesa_phone)
            .await?;
        // Payment reconciles the subscription status → bust cache
        self.cache.invalidate(agency_id).await;
        Ok(invoice_id)
    }
}
