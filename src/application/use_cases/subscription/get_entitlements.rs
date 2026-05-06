use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    domain::subscription::AgencyEntitlements,
    infrastructure::cache::subscription_cache::SubscriptionCache,
};

pub struct GetEntitlementsUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
}

impl GetEntitlementsUseCase {
    pub fn new(repo: Arc<dyn SubscriptionRepository>, cache: Arc<SubscriptionCache>) -> Self {
        Self { repo, cache }
    }

    pub async fn execute(&self, agency_id: Uuid) -> Result<AgencyEntitlements, AppError> {
        // Try cache (300 s TTL — matches NestJS)
        if let Some(cached) = self.cache.get_entitlements(agency_id).await {
            return Ok(cached);
        }
        // DB load
        let entitlements = self
            .repo
            .get_entitlements(agency_id)
            .await?
            .ok_or_else(|| {
                AppError::Forbidden("No active subscription found. Please contact support.".into())
            })?;
        // Store in cache
        self.cache.set_entitlements(agency_id, &entitlements).await;
        Ok(entitlements)
    }
}
