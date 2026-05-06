//! Bootstrap the notification workers.
//!
//! ## apalis 0.7.x WorkerBuilder rules
//!
//! - `Monitor::new()` — no generic parameter (0.7 removed it).
//! - Concurrency is achieved via `tower::limit::ConcurrencyLimitLayer`
//!   added with `.layer(...)` on the builder — the `"limit"` feature
//!   gates this layer. There is no `.concurrency()` shortcut method.
//! - `build_fn` is gone; use `.build(service_fn(handler))` instead.
//! - `.backend(storage)` must come *before* `.build(...)`.

use std::sync::Arc;

use anyhow::Context as AnyhowContext;
use apalis::prelude::{Monitor, WorkerBuilder, WorkerFactoryFn};
use apalis_redis::RedisStorage;
use minijinja::{path_loader, Environment};
use tower::limit::ConcurrencyLimitLayer;

use crate::application::{
    notifications::{
        jobs::{EmailJob, SmsJob},
        service::NotificationService,
    },
    ports::{email_port::EmailPort, sms_port::SmsPort},
};

use super::{email_worker::process_email_job, sms_worker::process_sms_job};

pub struct NotificationComponents {
    pub service: NotificationService,
    pub email_storage: RedisStorage<EmailJob>,
    pub sms_storage: RedisStorage<SmsJob>,
    pub env: Arc<Environment<'static>>,
}

pub async fn build_notification_components(
    templates_dir: &str,
    redis_url: &str,
) -> anyhow::Result<NotificationComponents> {
    let mut env = Environment::new();
    env.set_loader(path_loader(templates_dir));
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Lenient);

    let current_year = time::OffsetDateTime::now_utc().year();
    env.add_global("current_year", current_year);

    let env = Arc::new(env);
    tracing::info!(dir = templates_dir, "minijinja environment initialised ✓");

    let conn = apalis_redis::connect(redis_url)
        .await
        .context("apalis-redis connect failed")?;

    let email_storage: RedisStorage<EmailJob> = RedisStorage::new(conn.clone());
    let sms_storage: RedisStorage<SmsJob> = RedisStorage::new(conn);

    let service = NotificationService::new(email_storage.clone(), sms_storage.clone());

    Ok(NotificationComponents {
        service,
        email_storage,
        sms_storage,
        env,
    })
}

pub fn start_notification_workers(
    components: NotificationComponents,
    email_port: Arc<dyn EmailPort>,
    sms_port: Arc<dyn SmsPort>,
    concurrency: usize,
) -> tokio::task::JoinHandle<()> {
    let NotificationComponents {
        email_storage,
        sms_storage,
        env,
        ..
    } = components;

    tokio::spawn(async move {
        let mut monitor = Monitor::new(); // ← no generics in 0.7

        // ConcurrencyLimitLayer (from tower, gated by apalis "limit" feature)
        // caps how many jobs run simultaneously inside this single worker.
        let email_worker = WorkerBuilder::new("emakao-email-worker")
            .data(Arc::clone(&env))
            .data(Arc::clone(&email_port))
            .layer(ConcurrencyLimitLayer::new(concurrency))
            .backend(email_storage)
            .build_fn(process_email_job);

        monitor = monitor.register(email_worker);

        let sms_worker = WorkerBuilder::new("emakao-sms-worker")
            .data(Arc::clone(&sms_port))
            .layer(ConcurrencyLimitLayer::new(concurrency))
            .backend(sms_storage)
            .build_fn(process_sms_job);

        monitor = monitor.register(sms_worker);

        if let Err(err) = monitor.run().await {
            tracing::error!(%err, "notification monitor crashed");
        }
    })
}
