use rust_decimal::Decimal;
use std::sync::Arc;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            agreement_repository::{AgreementRepository, CreateAgreementCommand},
            openfga_port::OpenFgaPort,
        },
    },
    domain::{agreement::Agreement, enums::BillingFrequency, errors::DomainError},
};

pub struct CreateAgreementUseCase {
    pub repo: Arc<dyn AgreementRepository>,
    pub openfga: Arc<dyn OpenFgaPort>,
}

pub struct CreateAgreementInput {
    pub fga_store_id: Option<String>,
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
    pub fn new(repo: Arc<dyn AgreementRepository>, openfga: Arc<dyn OpenFgaPort>) -> Self {
        Self { repo, openfga }
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
        let agreement = self
            .repo
            .create(CreateAgreementCommand {
                property_id: input.property_id,
                unit_id: input.unit_id,
                resident_id: input.resident_id,
                start_date: input.start_date,
                end_date: input.end_date,
                rent_amount_kes: input.rent_amount_kes,
                deposit_kes: input.deposit_kes,
                billing_frequency: input.billing_frequency,
            })
            .await?;

        // ── OpenFGA Tuples ───────────────────────────────────────────────────
        //
        // 1. `unit:{id}` — parent_unit — `agreement:{id}`
        // 2. `user:{resident_id}` — signatory — `agreement:{id}`
        if let Some(ref store_id) = input.fga_store_id {
            let agreement_obj = format!("agreement:{}", agreement.id);
            let unit_obj = format!("unit:{}", input.unit_id);
            let resident_user = format!("user:{}", input.resident_id);

            // Parent link
            let _ = self
                .openfga
                .write_tuple(store_id, &unit_obj, "parent_unit", &agreement_obj)
                .await;

            // Resident is signatory
            let _ = self
                .openfga
                .write_tuple(store_id, &resident_user, "signatory", &agreement_obj)
                .await;

            tracing::info!(
                agreement_id = %agreement.id,
                store_id = %store_id,
                "OpenFGA tuples written for new agreement"
            );
        }

        tracing::info!(agreement_id = %agreement.id, unit_id = %input.unit_id, "agreement created");
        Ok(agreement)
    }
}
