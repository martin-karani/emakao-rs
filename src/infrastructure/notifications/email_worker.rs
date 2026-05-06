use std::sync::Arc;

use apalis::prelude::{Data, Error as JobError};
use minijinja::Environment;

use crate::application::{
    errors::AppError, notifications::jobs::EmailJob, ports::email_port::EmailPort,
};

fn job_err(e: AppError) -> JobError {
    JobError::Failed(Arc::new(Box::new(e)))
}

pub async fn process_email_job(
    job: EmailJob,
    env: Data<Arc<Environment<'static>>>,
    port: Data<Arc<dyn EmailPort>>,
) -> Result<(), JobError> {
    let env: &Arc<Environment<'static>> = &env;
    let email_port: &Arc<dyn EmailPort> = &port;

    let span = tracing::info_span!(
        "email_job",
        to       = %job.to,
        template = job.template.template_path(),
    );
    let _guard = span.enter();

    let tmpl = env
        .get_template(job.template.template_path())
        .map_err(|e| {
            tracing::error!(
                path = job.template.template_path(),
                "minijinja template not found: {e}"
            );
            job_err(AppError::ExternalService(format!(
                "template load '{}': {e}",
                job.template.template_path()
            )))
        })?;

    let html = tmpl.render(&job.context).map_err(|e| {
        tracing::error!(
            path = job.template.template_path(),
            "minijinja render failed: {e}"
        );
        job_err(AppError::ExternalService(format!(
            "template render '{}': {e}",
            job.template.template_path()
        )))
    })?;

    email_port
        .send(&job.to, job.template.subject(), &html)
        .await
        .map_err(|e| {
            tracing::error!(to = %job.to, "email send failed: {e}");
            job_err(AppError::ExternalService(e.to_string()))
        })?;

    tracing::info!(
        to       = %job.to,
        template = job.template.template_path(),
        "email delivered ✓"
    );
    Ok(())
}
