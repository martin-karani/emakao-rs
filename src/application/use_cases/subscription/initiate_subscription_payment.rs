use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    infrastructure::payments::mpesa_adapter::MpesaAdapter,
};

pub struct InitiateSubscriptionPaymentUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub mpesa: Arc<MpesaAdapter>,
}

pub struct InitiatePaymentInput {
    pub agency_id: Uuid,
    /// Must match a slug in `subscription_plans`.
    pub plan_slug: String,
    /// Safaricom-formatted phone number, e.g. "254712345678".
    pub phone_number: String,
}

pub struct InitiatePaymentOutput {
    /// Return this to the client so it can poll or display a status message.
    pub checkout_request_id: String,
    pub customer_message: String,
}

impl InitiateSubscriptionPaymentUseCase {
    pub async fn execute(
        &self,
        input: InitiatePaymentInput,
    ) -> Result<InitiatePaymentOutput, AppError> {
        // 1. Verify the plan exists and get its price
        let plan = self
            .repo
            .get_plan_by_slug(&input.plan_slug)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Plan '{}' not found", input.plan_slug)))?;

        let amount_kes = plan.price_kes as u64;

        // 2. Initiate STK Push
        let stk = self
            .mpesa
            .stk_push(
                &input.phone_number,
                amount_kes,
                &format!("SUB-{}", &input.agency_id.to_string()[..8]),
                &format!("eMakao {} subscription", plan.name),
            )
            .await?;

        // 3. Persist the pending request — idempotent on checkout_request_id
        self.repo
            .save_pending_mpesa_request(
                input.agency_id,
                &input.plan_slug,
                plan.price_kes,
                &stk.checkout_request_id,
                &stk.merchant_request_id,
            )
            .await?;

        tracing::info!(
            agency_id = %input.agency_id,
            plan_slug  = %input.plan_slug,
            checkout_id = %stk.checkout_request_id,
            "STK Push initiated for subscription payment"
        );

        Ok(InitiatePaymentOutput {
            checkout_request_id: stk.checkout_request_id,
            customer_message: stk.customer_message,
        })
    }
}
