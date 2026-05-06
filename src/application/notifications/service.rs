use crate::application::errors::AppError;
use apalis::prelude::Storage;
use apalis_redis::RedisStorage;
use serde::Serialize;

use super::{
    jobs::{EmailJob, SmsJob},
    templates::{EmailTemplate, SmsTemplate},
};

#[derive(Clone)]
pub struct NotificationService {
    email_queue: RedisStorage<EmailJob>,
    sms_queue: RedisStorage<SmsJob>,
}

impl NotificationService {
    pub fn new(email_queue: RedisStorage<EmailJob>, sms_queue: RedisStorage<SmsJob>) -> Self {
        Self {
            email_queue,
            sms_queue,
        }
    }

    /// Enqueue a transactional email. Rendering happens in the worker.
    pub async fn email(
        &self,
        to: impl Into<String>,
        template: EmailTemplate,
        ctx: impl Serialize,
    ) -> Result<(), AppError> {
        let job = EmailJob {
            to: to.into(),
            template,
            context: serde_json::to_value(ctx)
                .map_err(|e| AppError::InternalServer(format!("email ctx serialize: {e}")))?,
        };

        self.email_queue
            .clone()
            .push(job)
            .await
            .map(|_| ()) // apalis 0.7: push returns Parts<RedisContext>, discard it
            .map_err(|e| AppError::InternalServer(format!("email queue push: {e}")))
    }

    /// Render the SMS immediately via `SmsTemplate::render()`, then enqueue
    /// the pre-rendered string. The worker has no template engine dependency.
    pub async fn sms(
        &self,
        to: impl Into<String>,
        template: SmsTemplate,
        ctx: impl Serialize,
    ) -> Result<(), AppError> {
        let ctx_value = serde_json::to_value(ctx)
            .map_err(|e| AppError::InternalServer(format!("sms ctx serialize: {e}")))?;

        let message = template.render(&ctx_value)?;

        let job = SmsJob {
            to: to.into(),
            message,
        };

        self.sms_queue
            .clone()
            .push(job)
            .await
            .map(|_| ()) // apalis 0.7: push returns Parts<RedisContext>, discard it
            .map_err(|e| AppError::InternalServer(format!("sms queue push: {e}")))
    }

    // ── Convenience ───────────────────────────────────────────────────────────

    /// Enqueue both channels for the same event. Soft failures: if one push
    /// succeeds and the other fails, the success is kept and a warning is
    /// logged. Returns `Err` only when *both* fail.
    pub async fn email_and_sms(
        &self,
        email_to: impl Into<String> + Clone,
        email_template: EmailTemplate,
        sms_to: impl Into<String>,
        sms_template: SmsTemplate,
        ctx: impl Serialize + Clone,
    ) -> Result<(), AppError> {
        let email_res = self.email(email_to, email_template, ctx.clone()).await;
        let sms_res = self.sms(sms_to, sms_template, ctx).await;

        match (email_res, sms_res) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), Ok(())) => {
                tracing::warn!("email push failed: {e}");
                Ok(())
            }
            (Ok(()), Err(e)) => {
                tracing::warn!("sms push failed: {e}");
                Ok(())
            }
            (Err(ee), Err(es)) => Err(AppError::InternalServer(format!(
                "both notification pushes failed — email: {ee}, sms: {es}"
            ))),
        }
    }
}
