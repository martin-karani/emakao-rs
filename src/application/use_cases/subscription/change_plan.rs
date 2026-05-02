use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    domain::subscription::SubscriptionEvent,
    infrastructure::cache::subscription_cache::SubscriptionCache,
};

pub struct ChangePlanUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
    pub events: tokio::sync::broadcast::Sender<SubscriptionEvent>,
}

impl ChangePlanUseCase {
    pub async fn execute(
        &self,
        agency_id: Uuid,
        plan_slug: &str,
        mpesa_ref: Option<&str>,
        custom_price: Option<i32>,
    ) -> Result<(), AppError> {
        // Validate plan exists first
        self.repo
            .get_plan_by_slug(plan_slug)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Plan '{plan_slug}' not found")))?;

        self.repo
            .upsert_subscription(agency_id, plan_slug, "active", mpesa_ref, custom_price)
            .await?;

        // Bust cache so next request re-fetches fresh entitlements
        self.cache.invalidate(agency_id).await;

        let _ = self.events.send(SubscriptionEvent::Upgraded {
            agency_id,
            plan_slug: plan_slug.to_string(),
            mpesa_ref: mpesa_ref.map(String::from),
        });

        Ok(())
    }
}
