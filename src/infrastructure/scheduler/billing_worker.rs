use apalis::prelude::{Data, Monitor, WorkerBuilder, WorkerFactoryFn};
use apalis_redis::RedisStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::{
    notifications::service::NotificationService,
    ports::{
        billing_repository::BillingRepository,
        property_billing_repository::PropertyBillingRepository,
    },
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

    let repo = PgBillingRepo::for_agency(pool.clone(), job.agency_id);
    let property_billing_repo =
        crate::infrastructure::db::property_billing_repository_sqlx::PgPropertyBillingRepo::new(
            pool,
        );
    let today = OffsetDateTime::now_utc().date();

    let agreements = repo
        .find_active_agreements_for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("repo error: {e}"))?;

    for ag in agreements {
        // Load property settings for this agreement's property
        let settings = property_billing_repo
            .get_by_property_id(ag.property_id)
            .await
            .ok()
            .flatten();

        let (period_start, period_end) =
            match current_billing_period(ag.start_date, ag.billing_frequency, today) {
                Some(p) => p,
                None => continue,
            };

        if today != period_start {
            continue;
        }

        if repo
            .rent_charge_exists(ag.agreement_id, period_start)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            tracing::debug!(
                agreement_id = %ag.agreement_id,
                period_start = %period_start,
                "rent charge already exists — skipping"
            );
            continue;
        }

        // 1. Charge Rent
        repo.record_rent_charge(
            ag.agreement_id,
            period_start,
            period_end,
            ag.rent_amount_kes,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to record rent charge: {e}"))?;

        // 2. Charge Garbage Fee if configured
        if let Some(s) = &settings {
            if s.garbage_fee_kes > rust_decimal::Decimal::ZERO {
                repo.record_fixed_charge(
                    ag.agreement_id,
                    period_start,
                    s.garbage_fee_kes,
                    "Garbage Fee",
                )
                .await?;
            }

            // 3. Charge Security Fee if configured
            if s.security_fee_kes > rust_decimal::Decimal::ZERO {
                repo.record_fixed_charge(
                    ag.agreement_id,
                    period_start,
                    s.security_fee_kes,
                    "Security Fee",
                )
                .await?;
            }
        }

        tracing::info!(
            agreement_id = %ag.agreement_id,
            period_start = %period_start,
            period_end   = %period_end,
            amount_kes   = %ag.rent_amount_kes,
            "rent charged"
        );
    }

    Ok(())
}

pub async fn apply_late_fees_worker(
    job: ApplyLateFeesJob,
    ctx: Data<BillingContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let repo = PgBillingRepo::for_agency(pool.clone(), job.agency_id);
    let property_billing_repo =
        crate::infrastructure::db::property_billing_repository_sqlx::PgPropertyBillingRepo::new(
            pool,
        );
    let today = OffsetDateTime::now_utc().date();

    let agreements = repo
        .find_active_agreements_for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("repo error: {e}"))?;

    let agency_policy = repo.load_late_fee_policy().await?;

    for ag in agreements {
        let (period_start, _period_end) =
            match current_billing_period(ag.start_date, ag.billing_frequency, today) {
                Some(p) => p,
                None => continue,
            };

        // If we already charged a late fee for this period, skip
        if repo
            .late_fee_exists(ag.agreement_id, period_start)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            continue;
        }

        // Determine which policy to use
        let property_policy = property_billing_repo
            .get_by_property_id(ag.property_id)
            .await
            .ok()
            .flatten();

        let policy = if let Some(p_settings) = property_policy {
            let (flat, pct) = if p_settings.late_fee_type == "flat" {
                (Some(p_settings.late_fee_value), None)
            } else {
                (None, Some(p_settings.late_fee_value))
            };
            crate::domain::billing::LateFeePolicy {
                grace_period_days: p_settings.late_fee_grace_days as i64,
                flat_amount_kes: flat,
                rate_percent: pct,
            }
        } else {
            agency_policy.clone()
        };

        // Check if we are past the grace period
        let due_date = period_start + time::Duration::days(policy.grace_period_days);
        if today <= due_date {
            continue;
        }

        // Check if unpaid (simplified: check if total payments since period_start < rent_amount)
        // In a real app, we'd check the full ledger balance for that specific rent charge.
        let paid = repo
            .paid_amount_since(ag.agreement_id, period_start.midnight().assume_utc())
            .await?;

        if paid < ag.rent_amount_kes {
            let fee = policy.compute(ag.rent_amount_kes);
            if fee > rust_decimal::Decimal::ZERO {
                repo.record_late_fee(ag.agreement_id, period_start, fee)
                    .await?;

                tracing::info!(
                    agreement_id = %ag.agreement_id,
                    fee_kes = %fee,
                    "late fee applied"
                );
            }
        }
    }

    Ok(())
}

pub async fn send_reminders_worker(
    job: SendRemindersJob,
    _ctx: Data<BillingContext>,
) -> Result<(), anyhow::Error> {
    // ── AUTOMATIC REMINDERS DISABLED ───────────────────────────────────────
    //
    // As per user request, automatic reminders are disabled in favor of manual
    // statement broadcasts.
    tracing::debug!(agency_id = %job.agency_id, "automatic reminders suppressed (manual mode)");
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
