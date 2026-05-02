use std::sync::Arc;
use rust_decimal::Decimal;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::ledger_repository::LedgerRepository,
    },
    domain::ledger::{CreateLedgerEntryCommand, LedgerEntry, LedgerEntryType},
};

pub struct PostChargeUseCase {
    pub repo: Arc<dyn LedgerRepository>,
}

pub struct PostChargeInput {
    pub agreement_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub entry_type: LedgerEntryType,
    pub amount_kes: Decimal,
    pub description: String,
    pub external_ref: Option<String>,
    pub mpesa_receipt: Option<String>,
    pub period_start: Option<Date>,
    pub period_end: Option<Date>,
    pub posted_by: Uuid,
    pub metadata: serde_json::Value,
}

impl PostChargeUseCase {
    pub fn new(repo: Arc<dyn LedgerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: PostChargeInput) -> Result<LedgerEntry, AppError> {
        if input.amount_kes <= Decimal::ZERO {
            return Err(AppError::Validation("ledger amount must be positive".into()));
        }

        // Uses: LedgerRepository::create
        self.repo.create(CreateLedgerEntryCommand {
            agreement_id: input.agreement_id,
            unit_id: input.unit_id,
            resident_id: input.resident_id,
            owner_id: input.owner_id,
            entry_type: input.entry_type,
            amount_kes: input.amount_kes,
            description: input.description,
            external_ref: input.external_ref,
            mpesa_receipt: input.mpesa_receipt,
            period_start: input.period_start,
            period_end: input.period_end,
            posted_by: input.posted_by,
            metadata: input.metadata,
        }).await
    }
}