use async_trait::async_trait;
use rust_decimal::Decimal;
use time::Date;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::{
        agreement::Agreement,
        enums::{AgreementStatus, BillingFrequency},
    },
};

pub struct CreateAgreementCommand {
    pub property_id: Uuid,
    pub unit_id: Uuid,
    pub resident_id: Uuid,
    pub start_date: Date,
    pub end_date: Option<Date>,
    pub rent_amount_kes: Decimal,
    pub deposit_kes: Decimal,
    pub billing_frequency: BillingFrequency,
}

// ── Projections ───────────────────────────────────────────────────────────────

/// Lightweight view of an active agreement with owner context resolved.
///
/// Used exclusively by the tax compliance worker to compute MRI / VAT / WHT
/// obligations without loading the full Agreement + Property + Owner chain.
///
/// Produced by a single JOIN query:
///   agreements → units → properties → (owner_id, kra_pin)
#[derive(Debug, Clone)]
pub struct TaxAgreementView {
    /// `agreements.id`
    pub id: Uuid,
    /// `agreements.property_id` (= `units.property_id`)
    pub property_id: Uuid,
    /// `properties.owner_id`
    pub owner_id: Uuid,
    /// Owner's KRA PIN — needed for eRITS return payload.
    pub owner_kra_pin: Option<String>,
    pub rent_amount_kes: Decimal,
    pub billing_frequency: BillingFrequency,
    pub start_date: Date,
    pub end_date: Option<Date>,
}

// ── Port ──────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait AgreementRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        resident_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Agreement>, AppError>;

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Agreement>, AppError>;

    async fn create(&self, cmd: CreateAgreementCommand) -> Result<Agreement, AppError>;

    async fn update_status(
        &self,
        id: Uuid,
        status: AgreementStatus,
        updated_by: Uuid,
    ) -> Result<(), AppError>;

    async fn has_active_agreement(&self, unit_id: Uuid) -> Result<bool, AppError>;

    // ── NEW ───────────────────────────────────────────────────────────────────

    /// Returns all active agreements for the agency, with owner and property
    /// context resolved via JOIN.  Used by the tax compliance worker.
    ///
    /// Filters: `agreements.status = 'active'`
    async fn find_active_for_agency(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<TaxAgreementView>, AppError>;

    /// Returns the security deposit amount held for a given agreement.
    /// Used by the deposit refund use case.
    async fn get_deposit_kes(&self, agreement_id: Uuid) -> Result<Decimal, AppError>;
}
