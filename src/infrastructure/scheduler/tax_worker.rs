// src/infrastructure/scheduler/tax_worker.rs
//
// Three Apalis jobs:
//   GenerateTaxObligationsJob — runs on the 1st of each month;
//                               creates MRI / VAT / WHT rows for the closed period.
//   SweepOverdueTaxJob        — runs nightly; flips pending → overdue past due_date.
//   TaxDueReminderJob         — runs daily; sends email/SMS alerts at D-5 and D-1.

use apalis::prelude::{Data, Monitor, WorkerBuilder, WorkerFactoryFn};
use apalis_redis::RedisStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        notifications::service::NotificationService,
        ports::tax_repository::{TaxObligationFilter, TaxRepository},
        use_cases::tax::compute_obligations::{ComputeObligationsInput, ComputeObligationsUseCase},
    },
    domain::tax::{TaxObligationStatus, TaxPeriod},
    infrastructure::db::{pool::AgencyPoolManager, tax_repository_sqlx::PgTaxRepo},
};

// ── Worker context ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TaxWorkerContext {
    pub pool_manager: Arc<AgencyPoolManager>,
    pub notifications: NotificationService,
    /// Whether this agency is a KRA WHT agent.
    pub agency_is_wht_agent: bool,
    /// Whether agency is VAT-registered.
    pub agency_is_vat_registered: bool,
    /// Management fee rate (e.g. 0.10 = 10 %).
    pub management_fee_rate: rust_decimal::Decimal,
}

// ── Job definitions ────────────────────────────────────────────────────────

/// Enqueued on the 1st of each month to generate obligations for the
/// just-closed period (previous month).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateTaxObligationsJob {
    pub agency_id: Uuid,
    /// The closed period to compute (year + month of the PREVIOUS month).
    pub period_year: i32,
    pub period_month: u8,
}

/// Nightly job — sweeps past-due obligations to `overdue`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepOverdueTaxJob {
    pub agency_id: Uuid,
}

/// Daily job — sends email reminders for obligations due in 5 or 1 days.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxDueReminderJob {
    pub agency_id: Uuid,
}

// ── Workers ────────────────────────────────────────────────────────────────

pub async fn generate_tax_obligations_worker(
    job: GenerateTaxObligationsJob,
    ctx: Data<TaxWorkerContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let tax_repo = Arc::new(PgTaxRepo::new(pool.clone(), job.agency_id));
    let agreement_repo =
        Arc::new(crate::infrastructure::db::agreement_repository_sqlx::PgAgreementRepo::new(pool));

    let use_case = ComputeObligationsUseCase::new(agreement_repo, tax_repo);

    let result = use_case
        .execute(ComputeObligationsInput {
            agency_id: job.agency_id,
            tax_period: TaxPeriod::new(job.period_year, job.period_month),
            agency_is_wht_agent: ctx.agency_is_wht_agent,
            agency_is_vat_registered: ctx.agency_is_vat_registered,
            management_fee_rate: ctx.management_fee_rate,
        })
        .await
        .map_err(|e| anyhow::anyhow!("compute_obligations failed: {e:?}"))?;

    tracing::info!(
        agency_id = %job.agency_id,
        period    = "{}-{:02}", job.period_year, job.period_month,
        created   = result.obligations_created,
        skipped   = result.obligations_skipped,
        "tax obligations generated"
    );

    Ok(())
}

pub async fn sweep_overdue_tax_worker(
    job: SweepOverdueTaxJob,
    ctx: Data<TaxWorkerContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let repo: Arc<dyn TaxRepository> = Arc::new(PgTaxRepo::new(pool, job.agency_id));
    let today = OffsetDateTime::now_utc().date();

    let count = repo
        .mark_overdue_batch(job.agency_id, today)
        .await
        .map_err(|e| anyhow::anyhow!("mark_overdue_batch: {e:?}"))?;

    if count > 0 {
        tracing::warn!(
            agency_id = %job.agency_id,
            count     = count,
            "tax obligations marked overdue"
        );
    }

    Ok(())
}

pub async fn tax_due_reminder_worker(
    job: TaxDueReminderJob,
    ctx: Data<TaxWorkerContext>,
) -> Result<(), anyhow::Error> {
    let pool = ctx
        .pool_manager
        .for_agency(job.agency_id)
        .await
        .map_err(|e| anyhow::anyhow!("pool error: {e}"))?;

    let repo: Arc<dyn TaxRepository> = Arc::new(PgTaxRepo::new(pool, job.agency_id));
    let today = OffsetDateTime::now_utc().date();

    // Check obligations due in 5 days or 1 day
    for days_ahead in [5i64, 1i64] {
        let target_due = today + time::Duration::days(days_ahead);

        let obligations = repo
            .list_obligations(TaxObligationFilter {
                agency_id: job.agency_id,
                owner_id: None,
                property_id: None,
                obligation_type: None,
                status: Some(TaxObligationStatus::Pending),
                tax_period: None,
                due_on_or_before: Some(target_due),
                limit: 500,
                offset: 0,
            })
            .await
            .map_err(|e| anyhow::anyhow!("list_obligations: {e:?}"))?;

        for ob in obligations.iter().filter(|o| o.due_date == target_due) {
            // TODO: look up the owner's email from the owner repo and send
            // a real per-owner notification.  For now we log.
            tracing::info!(
                obligation_id  = %ob.id,
                obligation_type = %ob.obligation_type,
                tax_kes        = %ob.tax_kes,
                due_date       = %ob.due_date,
                days_until_due = days_ahead,
                "tax due reminder would fire here"
            );

            // Placeholder — replace with real owner email lookup + send:
            // ctx.notifications.email(&owner_email, EmailTemplate::TaxDue, TaxDueCtx { ... }).await?;
        }
    }

    Ok(())
}

// ── Monitor builder ────────────────────────────────────────────────────────

pub async fn build_tax_monitor(
    ctx: TaxWorkerContext,
    redis_url: String,
) -> anyhow::Result<Monitor> {
    let conn = apalis_redis::connect(redis_url).await?;

    let gen_storage = RedisStorage::new(conn.clone());
    let sweep_storage = RedisStorage::new(conn.clone());
    let reminder_storage = RedisStorage::new(conn);

    let monitor = Monitor::new()
        .register(
            WorkerBuilder::new("generate-tax-obligations")
                .data(ctx.clone())
                .backend(gen_storage)
                .build_fn(generate_tax_obligations_worker),
        )
        .register(
            WorkerBuilder::new("sweep-overdue-tax")
                .data(ctx.clone())
                .backend(sweep_storage)
                .build_fn(sweep_overdue_tax_worker),
        )
        .register(
            WorkerBuilder::new("tax-due-reminder")
                .data(ctx)
                .backend(reminder_storage)
                .build_fn(tax_due_reminder_worker),
        );

    Ok(monitor)
}
