use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    domain::subscription::SubscriptionInvoice,
};

pub struct ListInvoicesUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
}

impl ListInvoicesUseCase {
    pub async fn execute(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SubscriptionInvoice>, AppError> {
        self.repo.list_invoices(agency_id, limit, offset).await
    }
}
