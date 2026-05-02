use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    infrastructure::cache::subscription_cache::SubscriptionCache,
};

pub struct SetFeatureOverrideUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
}

impl SetFeatureOverrideUseCase {
    pub async fn execute(
        &self,
        agency_id: Uuid,
        feature_key: &str,
        value: &str,
        reason: Option<&str>,
        created_by: Option<Uuid>,
        expires_at: Option<OffsetDateTime>,
    ) -> Result<(), AppError> {
        self.repo
            .set_feature_override(
                agency_id,
                feature_key,
                value,
                reason,
                created_by,
                expires_at,
            )
            .await?;
        self.cache.invalidate(agency_id).await;
        Ok(())
    }
}
