// src/application/ports/billing_repository.rs
//
// Port used by the billing scheduler to read active agreements and
// record rent charges, late fees, and sent reminders.

use async_trait::async_trait;
use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{application::errors::AppError, domain::agreement::ActiveAgreementBillingView};

#[async_trait]
pub trait BillingRepository: Send + Sync + 'static {
    // ── Active agreements ─────────────────────────────────────────────────────

    /// Returns all active agreements across all agencies (used by the global
    /// scheduler). Each row carries enough data to run billing without joining.
    async fn find_active_agreements_for_agency(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<ActiveAgreementBillingView>, AppError>;

    // ── Rent charges ──────────────────────────────────────────────────────────

    /// Returns true when a rent charge row already exists for this
    /// (agreement, period_start) — idempotency guard.
    async fn rent_charge_exists(
        &self,
        agreement_id: Uuid,
        period_start: Date,
    ) -> Result<bool, AppError>;

    /// Inserts a rent_charge row. If the UNIQUE constraint fires (scheduler
    /// retry race), returns Ok(()) silently — the charge was already recorded.
    async fn record_rent_charge(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        period_end: Date,
        amount_kes: Decimal,
    ) -> Result<(), AppError>;

    // ── Late fees ─────────────────────────────────────────────────────────────

    /// Returns true when a late_fee_charge row already exists for this
    /// (agreement, period_start).
    async fn late_fee_exists(
        &self,
        agreement_id: Uuid,
        period_start: Date,
    ) -> Result<bool, AppError>;

    /// Inserts a late_fee_charge row.
    async fn record_late_fee(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        amount_kes: Decimal,
    ) -> Result<(), AppError>;

    /// Returns the sum of payments credited to the ledger for this agreement
    /// since period_start (used to decide whether a late fee applies).
    async fn paid_amount_since(
        &self,
        agreement_id: Uuid,
        since: OffsetDateTime,
    ) -> Result<Decimal, AppError>;

    // ── Reminders ─────────────────────────────────────────────────────────────

    /// Returns true when a reminder was already sent via `channel` for this
    /// (agreement, period_start).
    async fn reminder_sent(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        channel: &str,
    ) -> Result<bool, AppError>;

    /// Records that a reminder was sent.
    async fn record_reminder_sent(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        channel: &str,
    ) -> Result<(), AppError>;

    // ── Late fee policy ───────────────────────────────────────────────────────

    /// Loads the agency's late fee policy (grace period days, flat/% amount).
    async fn load_late_fee_policy(&self) -> Result<LateFeePolicy, AppError>;
}

#[derive(Debug, Clone)]
pub struct LateFeePolicy {
    pub grace_period_days: i32,
    pub flat_amount_kes: Option<Decimal>,
    pub rate_percent: Option<Decimal>,
}

impl LateFeePolicy {
    pub fn compute_fee(&self, rent_kes: Decimal) -> Decimal {
        if let Some(flat) = self.flat_amount_kes {
            return flat;
        }
        if let Some(rate) = self.rate_percent {
            return (rent_kes * rate / Decimal::from(100)).round_dp(2);
        }
        Decimal::ZERO
    }
}
