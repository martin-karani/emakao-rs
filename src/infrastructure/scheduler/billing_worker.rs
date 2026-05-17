use apalis::prelude::{Data, Monitor, WorkerBuilder, WorkerFactoryFn};
use apalis_redis::RedisStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::application::{
    notifications::{
        service::NotificationService,
        templates::{EmailTemplate, SmsTemplate},
    },
    ports::billing_repository::BillingRepository,
};
use crate::domain::billing::current_billing_period;
use crate::infrastructure::db::billing_repository_sqlx::PgBillingRepo;
use crate::infrastructure::db::pool::AgencyPoolManager;

// ── Worker context ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BillingContext {
    pub pool_manager: Arc<AgencyPoolManager>,
    pub notifications: NotificationService,
}

// ── Jobs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargeRentJob {
    pub agency_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyLateFeesJob {
    pub agency_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRemindersJob {
    pub agency_id: Uuid,
}

// ── Workers ────────────────────────────────────────────────────────────────

pub async fn charge_rent_worker(
    job: ChargeRentJob,
    ctx: Data<BillingContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let repo = PgBillingRepo::new(pool);
    let today = OffsetDateTime::now_utc().date();

    let agreements = repo
        .find_active_agreements_for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("repo error: {e}"))?;

    for ag in agreements {
        let (period_start, period_end) =
            match current_billing_period(ag.start_date, ag.billing_frequency, today) {
                Some((ps, pe)) => (ps, pe),
                None => continue,
            };

        if today != period_start {
            continue;
        }

        if !repo
            .rent_charge_exists(ag.agreement_id, period_start)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            repo.record_rent_charge(
                ag.agreement_id,
                period_start,
                period_end,
                ag.rent_amount_kes,
            )
            .await
            .map_err(|e| anyhow::anyhow!("failed to record rent charge: {e}"))?;

            tracing::info!(agreement_id = %ag.agreement_id, "rent charged for period {} to {}", period_start, period_end);
        }
    }

    Ok(())
}

pub async fn apply_late_fees_worker(
    job: ApplyLateFeesJob,
    _ctx: Data<BillingContext>,
) -> Result<(), anyhow::Error> {
    // TODO: Implement late fee logic
    tracing::info!(agency_id = %job.agency_id, "apply_late_fees_worker stub called");
    Ok(())
}

pub async fn send_reminders_worker(
    job: SendRemindersJob,
    ctx: Data<BillingContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let repo = PgBillingRepo::new(pool);
    let today = OffsetDateTime::now_utc().date();

    let agreements = repo
        .find_active_agreements_for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("repo error: {e}"))?;

    for ag in agreements {
        let (period_start, _) =
            match current_billing_period(ag.start_date, ag.billing_frequency, today) {
                Some((ps, pe)) => (ps, pe),
                None => continue,
            };

        let remind_on = period_start - Duration::days(3);
        if today != remind_on {
            continue;
        }

        // ── Email reminder ─────────────────────────────────────────────────

        if let Some(ref email) = ag.resident_email {
            if !repo
                .reminder_sent(ag.agreement_id, period_start, "email")
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?
            {
                let result = ctx
                    .notifications
                    .email(
                        email,
                        EmailTemplate::RentDue,
                        serde_json::json!({
                            "resident_name": ag.resident_first_name,
                            "amount_kes":    ag.rent_amount_kes.to_string(),
                            "unit_ref":      ag.unit_number,
                            "due_date":      period_start.to_string(),
                            "payment_url":   "",
                        }),
                    )
                    .await;

                match result {
                    Ok(_) => {
                        repo.record_reminder_sent(ag.agreement_id, period_start, "email")
                            .await
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                        tracing::info!(agreement_id = %ag.agreement_id, channel = "email", "rent reminder enqueued");
                    }
                    Err(e) => {
                        tracing::warn!(agreement_id = %ag.agreement_id, err = %e, "failed to enqueue email reminder");
                    }
                }
            }
        }

        // ── SMS reminder ───────────────────────────────────────────────────

        if let Some(ref phone) = ag.resident_phone {
            if !repo
                .reminder_sent(ag.agreement_id, period_start, "sms")
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?
            {
                let result = ctx
                    .notifications
                    .sms(
                        phone,
                        SmsTemplate::RentDue,
                        serde_json::json!({
                            "resident_name": ag.resident_first_name,
                            "amount_kes":    ag.rent_amount_kes.to_string(),
                            "unit_ref":      ag.unit_number,
                            "due_date":      period_start.to_string(),
                            "payment_url":   "",
                        }),
                    )
                    .await;

                match result {
                    Ok(_) => {
                        repo.record_reminder_sent(ag.agreement_id, period_start, "sms")
                            .await
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                        tracing::info!(agreement_id = %ag.agreement_id, channel = "sms", "rent reminder enqueued");
                    }
                    Err(e) => {
                        tracing::warn!(agreement_id = %ag.agreement_id, err = %e, "failed to enqueue sms reminder");
                    }
                }
            }
        }
    }

    Ok(())
}

// ── Monitor builder ────────────────────────────────────────────────────────

pub async fn build_billing_monitor(
    ctx: BillingContext,
    redis_url: String,
) -> anyhow::Result<Monitor> {
    let conn = apalis_redis::connect(redis_url).await?;

    let charge_storage = RedisStorage::new(conn.clone());
    let late_fee_storage = RedisStorage::new(conn.clone());
    let reminder_storage = RedisStorage::new(conn);

    let monitor = Monitor::new()
        .register(
            WorkerBuilder::new("charge-rent")
                .data(ctx.clone())
                .backend(charge_storage)
                .build_fn(charge_rent_worker),
        )
        .register(
            WorkerBuilder::new("apply-late-fees")
                .data(ctx.clone())
                .backend(late_fee_storage)
                .build_fn(apply_late_fees_worker),
        )
        .register(
            WorkerBuilder::new("send-reminders")
                .data(ctx)
                .backend(reminder_storage)
                .build_fn(send_reminders_worker),
        );

    Ok(monitor)
}
