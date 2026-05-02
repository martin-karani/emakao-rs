use std::sync::Arc;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    domain::subscription::SubscriptionPlan,
};

pub struct ListPlansUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
}

impl ListPlansUseCase {
    pub fn new(repo: Arc<dyn SubscriptionRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<Vec<SubscriptionPlan>, AppError> {
        self.repo.list_plans().await
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<SubscriptionPlan, AppError> {
        self.repo
            .get_plan_by_slug(slug)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Plan '{slug}' not found")))
    }
}
