use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    domain::subscription::SubscriptionEvent,
    infrastructure::cache::subscription_cache::SubscriptionCache,
};

pub struct CancelSubscriptionUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
    pub events: tokio::sync::broadcast::Sender<SubscriptionEvent>,
}

impl CancelSubscriptionUseCase {
    pub async fn execute(&self, agency_id: Uuid, reason: Option<&str>) -> Result<(), AppError> {
        self.repo.cancel_subscription(agency_id, reason).await?;
        self.cache.invalidate(agency_id).await;
        let _ = self.events.send(SubscriptionEvent::Cancelled {
            agency_id,
            reason: reason.map(String::from),
        });
        Ok(())
    }
}
