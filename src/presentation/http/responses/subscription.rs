use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::subscription::SubscriptionPlan;

#[derive(Serialize, ToSchema)]
pub struct SubscriptionStatusResponse {
    pub is_active: bool,
    pub is_trial: bool,
    pub is_paid: bool,
    pub in_grace_period: bool,
    pub trial_days_remaining: i64,
    pub days_until_renewal: i64,
    pub plan_slug: String,
    pub plan_name: String,
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct PlanResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub price_kes: i32,
    pub yearly_price_kes: Option<i32>,
    pub trial_days: i32,
    pub sort_order: i32,
    #[schema(value_type = Object, nullable = true)]
    pub metadata: Option<serde_json::Value>,
}

impl From<SubscriptionPlan> for PlanResponse {
    fn from(p: SubscriptionPlan) -> Self {
        Self {
            id: p.id,
            slug: p.slug,
            name: p.name,
            description: p.description,
            price_kes: p.price_kes,
            yearly_price_kes: p.yearly_price_kes,
            trial_days: p.trial_days,
            sort_order: p.sort_order,
            metadata: p.metadata,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct InitiatePaymentResponse {
    pub checkout_request_id: String,
    pub message: String,
}
