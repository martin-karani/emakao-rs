use std::sync::Arc;
use rust_decimal::Decimal;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::agreement_repository::{AgreementRepository, CreateAgreementCommand},
    },
    domain::{
        agreement::{Agreement, BillingFrequency},
        errors::DomainError,
    },
};

pub struct CreateAgreementUseCase {
    pub repo: Arc<dyn AgreementRepository>,
}

pub struct CreateAgreementInput {
    pub property_id: Uuid,
    pub unit_id: Uuid,
    pub resident_id: Uuid,
    pub start_date: Date,
    pub end_date: Option<Date>,
    pub rent_amount_kes: Decimal,
    pub deposit_kes: Decimal,
    pub billing_frequency: BillingFrequency,
}

impl CreateAgreementUseCase {
    pub fn new(repo: Arc<dyn AgreementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateAgreementInput) -> Result<Agreement, AppError> {
        if input.rent_amount_kes <= Decimal::ZERO {
            return Err(AppError::Validation("rent amount must be positive".into()));
        }

        // Uses: AgreementRepository::has_active_agreement
        if self.repo.has_active_agreement(input.unit_id).await? {
            return Err(DomainError::UnitOccupied(input.unit_id).into());
        }

        // Uses: AgreementRepository::create
        let agreement = self.repo.create(CreateAgreementCommand {
            property_id: input.property_id,
            unit_id: input.unit_id,
            resident_id: input.resident_id,
            start_date: input.start_date,
            end_date: input.end_date,
            rent_amount_kes: input.rent_amount_kes,
            deposit_kes: input.deposit_kes,
            billing_frequency: input.billing_frequency,
        }).await?;

        tracing::info!(agreement_id = %agreement.id, unit_id = %input.unit_id, "agreement created");
        Ok(agreement)
    }
}