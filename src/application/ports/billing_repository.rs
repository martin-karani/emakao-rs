use async_trait::async_trait;
use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    // Single canonical source — supersedes domain::agreement::ActiveAgreementBillingView
    domain::billing::{ActiveAgreementBillingView, BillingSettings, BillingSummary, LateFeePolicy},
};

#[async_trait]
pub trait BillingRepository: Send + Sync {
    async fn find_active_agreements_for_agency(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<ActiveAgreementBillingView>, AppError>;

    async fn rent_charge_exists(
        &self,
        agreement_id: Uuid,
        period_start: Date,
    ) -> Result<bool, AppError>;

    async fn record_rent_charge(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        period_end: Date,
        amount_kes: Decimal,
    ) -> Result<(), AppError>;

    async fn record_fixed_charge(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        amount_kes: Decimal,
        description: &str,
    ) -> Result<(), AppError>;

    async fn load_late_fee_policy(&self) -> Result<LateFeePolicy, AppError>;

    async fn load_late_fee_policy_for_property(
        &self,
        property_id: Uuid,
    ) -> Result<Option<LateFeePolicy>, AppError>;

    async fn late_fee_exists(
        &self,
        agreement_id: Uuid,
        period_start: Date,
    ) -> Result<bool, AppError>;

    async fn record_late_fee(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        fee_kes: Decimal,
    ) -> Result<(), AppError>;

    async fn paid_amount_since(
        &self,
        agreement_id: Uuid,
        since: OffsetDateTime,
    ) -> Result<Decimal, AppError>;

    async fn reminder_sent(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        channel: &str,
    ) -> Result<bool, AppError>;

    async fn record_reminder_sent(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        channel: &str,
    ) -> Result<(), AppError>;

    // Property billing settings methods
    async fn get_billing_settings_by_property_id(
        &self,
        property_id: Uuid,
    ) -> Result<Option<BillingSettings>, AppError>;

    async fn upsert_billing_settings(&self, settings: BillingSettings) -> Result<(), AppError>;

    async fn get_billing_summary_for_agency(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<BillingSummary>, AppError>;
}
