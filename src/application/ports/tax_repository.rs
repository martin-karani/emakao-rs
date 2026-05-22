// src/application/ports/tax_repository.rs

use async_trait::async_trait;
use time::Date;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::tax::{
        CreateTaxObligationCommand, MarkTaxFiledCommand, MarkTaxPaidCommand, MriRegime,
        OwnerAnnualRentalIncome, TaxComplianceSummary, TaxObligation, TaxObligationStatus,
        TaxObligationType, TaxPeriod,
    },
};

// ── Filters ───────────────────────────────────────────────────────────────────

pub struct TaxObligationFilter {
    pub agency_id: Uuid,
    pub owner_id: Option<Uuid>,
    pub property_id: Option<Uuid>,
    pub obligation_type: Option<TaxObligationType>,
    pub status: Option<TaxObligationStatus>,
    /// Filter to a specific period (first day of month).
    pub tax_period: Option<Date>,
    /// Return obligations due on or before this date (for overdue scanning).
    pub due_on_or_before: Option<Date>,
    pub limit: i64,
    pub offset: i64,
}

// ── Port ──────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait TaxRepository: Send + Sync + 'static {
    /// Persist a new tax obligation (idempotent via the UNIQUE constraint).
    async fn create_obligation(
        &self,
        cmd: CreateTaxObligationCommand,
    ) -> Result<TaxObligation, AppError>;

    /// Retrieve a single obligation by primary key.
    async fn find_obligation_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<TaxObligation>, AppError>;

    /// List obligations with optional filters.
    async fn list_obligations(
        &self,
        filter: TaxObligationFilter,
    ) -> Result<Vec<TaxObligation>, AppError>;

    /// Mark an obligation as filed with KRA (eRITS / iTax ack received).
    async fn mark_filed(&self, cmd: MarkTaxFiledCommand) -> Result<TaxObligation, AppError>;

    /// Mark an obligation as paid (PRN confirmed).
    async fn mark_paid(&self, cmd: MarkTaxPaidCommand) -> Result<TaxObligation, AppError>;

    /// Bulk-update all obligations that are past their due_date and still
    /// `pending` or `filed` to `overdue`.  Called nightly by the tax worker.
    async fn mark_overdue_batch(&self, agency_id: Uuid, as_of: Date) -> Result<u64, AppError>;

    /// Upsert the owner's rolling annual gross rent figure.
    async fn upsert_annual_income(
        &self,
        agency_id: Uuid,
        owner_id: Uuid,
        year: i32,
        total_gross_rent_kes: rust_decimal::Decimal,
        regime: MriRegime,
    ) -> Result<OwnerAnnualRentalIncome, AppError>;

    /// Fetch the owner's annual income record for a given year.
    async fn get_annual_income(
        &self,
        agency_id: Uuid,
        owner_id: Uuid,
        year: i32,
    ) -> Result<Option<OwnerAnnualRentalIncome>, AppError>;

    /// Aggregated compliance summary for the compliance dashboard widget.
    async fn get_compliance_summary(
        &self,
        agency_id: Uuid,
        tax_period: TaxPeriod,
    ) -> Result<TaxComplianceSummary, AppError>;

    /// Check if an obligation already exists (avoids duplicate creation).
    async fn obligation_exists(
        &self,
        agency_id: Uuid,
        owner_id: Uuid,
        property_id: Uuid,
        tax_period: Date,
        obligation_type: TaxObligationType,
    ) -> Result<bool, AppError>;
}
