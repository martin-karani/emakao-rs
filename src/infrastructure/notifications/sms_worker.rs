use std::sync::Arc;

use apalis::prelude::{Data, Error as JobError};

use crate::application::{errors::AppError, notifications::jobs::SmsJob, ports::sms_port::SmsPort};

fn job_err(e: AppError) -> JobError {
    JobError::Failed(Arc::new(Box::new(e)))
}

pub async fn process_sms_job(
    job: SmsJob,
    sms_port: Data<Arc<dyn SmsPort>>,
) -> Result<(), JobError> {
    let sms_port: &Arc<dyn SmsPort> = &sms_port;

    let span = tracing::info_span!("sms_job", to = %job.to);
    let _guard = span.enter();

    if job.message.len() > 160 {
        tracing::warn!(
            len = job.message.len(),
            "SMS exceeds 160 chars; provider will split into multiple segments"
        );
    }

    sms_port
        .send(&job.to, job.message.trim())
        .await
        .map_err(|e| {
            tracing::error!(to = %job.to, "sms send failed: {e}");
            job_err(AppError::ExternalService(e.to_string()))
        })?;

    tracing::info!(to = %job.to, "SMS delivered ✓");
    Ok(())
}
