use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    domain::subscription::SubscriptionState,
    infrastructure::cache::subscription_cache::SubscriptionCache,
};

pub struct GetSubscriptionStateUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
}

impl GetSubscriptionStateUseCase {
    pub fn new(repo: Arc<dyn SubscriptionRepository>, cache: Arc<SubscriptionCache>) -> Self {
        Self { repo, cache }
    }

    pub async fn execute(&self, agency_id: Uuid) -> Result<SubscriptionState, AppError> {
        if let Some(cached) = self.cache.get_state(agency_id).await {
            return Ok(cached);
        }
        let state = self
            .repo
            .get_state(agency_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("No active subscription found.".into()))?;
        self.cache.set_state(agency_id, &state).await;
        Ok(state)
    }
}
