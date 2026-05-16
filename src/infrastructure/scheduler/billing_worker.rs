// src/infrastructure/scheduler/billing_worker.rs
//
// Apalis-based background jobs for billing.
//
// Three job types run on cron schedules:
//   1. ChargeRentJob       — daily at 00:15, raises rent charges for any
//                            agreement whose billing day falls today.
//   2. ApplyLateFeesJob    — daily at 08:00, checks for unpaid charges past
//                            the grace period and records late fees.
//   3. SendRemindersJob    — daily at 09:00, sends email + SMS reminders for
//                            upcoming rent due.
//
// All three are idempotent: they use ON CONFLICT DO NOTHING in the DB.

use apalis::{
    cron::{CronStream, Schedule},
    layers::tracing::TraceLayer,
    prelude::{Monitor, WorkerBuilder, WorkerFactoryFn},
};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use time::{Duration, OffsetDateTime};

use crate::{
    application::ports::{
        billing_repository::{BillingRepository, LateFeePolicy},
        notification_port::NotificationPort,
    },
    domain::agreement::{billing_frequency::BillingFrequency, current_billing_period},
    infrastructure::db::billing_repository_sqlx::PgBillingRepo,
    presentation::app_state::AgencyPoolManager,
};

// ── Job types ─────────────────────────────────────────────────────────────────
// Each job carries an agency_id so the worker opens the right per-agency pool.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargeRentJob {
    pub agency_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyLateFeesJob {
    pub agency_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRemindersJob {
    pub agency_id: uuid::Uuid,
}

// ── Worker context ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BillingContext {
    pub pool_manager: Arc<AgencyPoolManager>,
    pub notifications: Arc<dyn NotificationPort>,
}

// ── Worker: ChargeRent ────────────────────────────────────────────────────────

pub async fn charge_rent_worker(
    job: ChargeRentJob,
    ctx: apalis::prelude::Data<BillingContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .get_or_create(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let repo = PgBillingRepo::new(pool);
    let today = OffsetDateTime::now_utc().date();

    let agreements = repo
        .find_active_agreements_for_agency(job.agency_id)
        .await?;

    for ag in agreements {
        let freq: BillingFrequency = ag
            .billing_frequency
            .parse()
            .unwrap_or(BillingFrequency::Monthly);

        let (period_start, period_end) = current_billing_period(today, ag.billing_day, &freq);

        // Only charge on the first day of the period
        if period_start != today {
            continue;
        }

        if repo
            .rent_charge_exists(ag.agreement_id, period_start)
            .await?
        {
            tracing::debug!(
                agreement_id = %ag.agreement_id,
                period_start = %period_start,
                "rent charge already exists — skipping"
            );
            continue;
        }

        repo.record_rent_charge(
            ag.agreement_id,
            period_start,
            period_end,
            ag.rent_amount_kes,
        )
        .await?;

        tracing::info!(
            agreement_id = %ag.agreement_id,
            period_start = %period_start,
            amount_kes   = %ag.rent_amount_kes,
            "rent charge recorded"
        );
    }

    Ok(())
}

// ── Worker: ApplyLateFees ─────────────────────────────────────────────────────

pub async fn apply_late_fees_worker(
    job: ApplyLateFeesJob,
    ctx: apalis::prelude::Data<BillingContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .get_or_create(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let repo = PgBillingRepo::new(pool);
    let policy = repo.load_late_fee_policy().await?;
    let today = OffsetDateTime::now_utc().date();

    let agreements = repo
        .find_active_agreements_for_agency(job.agency_id)
        .await?;

    for ag in agreements {
        let freq: BillingFrequency = ag
            .billing_frequency
            .parse()
            .unwrap_or(BillingFrequency::Monthly);

        let (period_start, _) = current_billing_period(today, ag.billing_day, &freq);

        // Late fee check only applies once grace period has passed
        let grace_deadline = period_start + Duration::days(policy.grace_period_days as i64);

        if today < grace_deadline {
            continue;
        }

        if repo.late_fee_exists(ag.agreement_id, period_start).await? {
            continue;
        }

        // Was enough paid since period start?
        let since = period_start.midnight().assume_utc();
        let paid = repo.paid_amount_since(ag.agreement_id, since).await?;

        if paid >= ag.rent_amount_kes {
            tracing::debug!(
                agreement_id = %ag.agreement_id,
                "rent paid in full — no late fee"
            );
            continue;
        }

        let fee = policy.compute_fee(ag.rent_amount_kes);
        if fee <= rust_decimal::Decimal::ZERO {
            continue;
        }

        repo.record_late_fee(ag.agreement_id, period_start, fee)
            .await?;

        tracing::info!(
            agreement_id = %ag.agreement_id,
            fee_kes      = %fee,
            "late fee recorded"
        );
    }

    Ok(())
}

// ── Worker: SendReminders ─────────────────────────────────────────────────────

pub async fn send_reminders_worker(
    job: SendRemindersJob,
    ctx: apalis::prelude::Data<BillingContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .get_or_create(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let repo = PgBillingRepo::new(pool);
    let today = OffsetDateTime::now_utc().date();

    let agreements = repo
        .find_active_agreements_for_agency(job.agency_id)
        .await?;

    for ag in agreements {
        let freq: BillingFrequency = ag
            .billing_frequency
            .parse()
            .unwrap_or(BillingFrequency::Monthly);

        let (period_start, _) = current_billing_period(today, ag.billing_day, &freq);

        // Send reminder 3 days before the billing day
        let remind_on = period_start - Duration::days(3);
        if today != remind_on {
            continue;
        }

        for channel in ["email", "sms"] {
            if repo
                .reminder_sent(ag.agreement_id, period_start, channel)
                .await?
            {
                continue;
            }

            let result = match channel {
                "email" => {
                    ctx.notifications
                        .send_rent_reminder_email(
                            &ag.resident_email,
                            &ag.resident_name,
                            ag.rent_amount_kes,
                            period_start,
                        )
                        .await
                }
                "sms" => {
                    ctx.notifications
                        .send_rent_reminder_sms(
                            &ag.resident_phone,
                            &ag.resident_name,
                            ag.rent_amount_kes,
                            period_start,
                        )
                        .await
                }
                _ => unreachable!(),
            };

            match result {
                Ok(_) => {
                    repo.record_reminder_sent(ag.agreement_id, period_start, channel)
                        .await?;
                    tracing::info!(
                        agreement_id = %ag.agreement_id,
                        channel      = channel,
                        "rent reminder sent"
                    );
                }
                Err(e) => {
                    // Log but don't fail the job — a missed reminder shouldn't
                    // block the whole run
                    tracing::warn!(
                        agreement_id = %ag.agreement_id,
                        channel      = channel,
                        err          = %e,
                        "failed to send rent reminder"
                    );
                }
            }
        }
    }

    Ok(())
}

// ── Monitor registration ──────────────────────────────────────────────────────
//
// Call `build_billing_monitor` from main.rs and `.run()` it alongside the Axum
// server with `tokio::spawn`.
//
// Example main.rs integration:
//
//   let billing_monitor = build_billing_monitor(billing_ctx, redis_storage.clone()).await?;
//   tokio::spawn(billing_monitor.run());
//   axum::serve(listener, router).await?;

pub async fn build_billing_monitor(
    ctx: BillingContext,
    redis_conn: apalis_redis::RedisStorage<ChargeRentJob>,
) -> anyhow::Result<Monitor> {
    let ctx = Arc::new(ctx);

    // Cron: daily at 00:15 — charge rent
    let rent_schedule = Schedule::from_str("15 0 * * *")?;
    // Cron: daily at 08:00 — apply late fees
    let late_schedule = Schedule::from_str("0 8 * * *")?;
    // Cron: daily at 09:00 — send reminders
    let remind_schedule = Schedule::from_str("0 9 * * *")?;

    let monitor = Monitor::new()
        .register_with_count(
            1,
            WorkerBuilder::new("charge-rent")
                .layer(TraceLayer::new())
                .data((*ctx).clone())
                .stream(CronStream::new(rent_schedule).into_stream())
                .build_fn(charge_rent_worker),
        )
        .register_with_count(
            1,
            WorkerBuilder::new("apply-late-fees")
                .layer(TraceLayer::new())
                .data((*ctx).clone())
                .stream(CronStream::new(late_schedule).into_stream())
                .build_fn(apply_late_fees_worker),
        )
        .register_with_count(
            1,
            WorkerBuilder::new("send-reminders")
                .layer(TraceLayer::new())
                .data((*ctx).clone())
                .stream(CronStream::new(remind_schedule).into_stream())
                .build_fn(send_reminders_worker),
        );

    Ok(monitor)
}
