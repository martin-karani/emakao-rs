use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    infrastructure::cache::subscription_cache::SubscriptionCache,
};

pub struct RemoveFeatureOverrideUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
}

impl RemoveFeatureOverrideUseCase {
    pub async fn execute(&self, agency_id: Uuid, feature_key: &str) -> Result<(), AppError> {
        self.repo
            .remove_feature_override(agency_id, feature_key)
            .await?;
        self.cache.invalidate(agency_id).await;
        Ok(())
    }
}
