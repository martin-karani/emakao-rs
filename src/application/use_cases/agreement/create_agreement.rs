use rust_decimal::Decimal;
use std::sync::Arc;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            agreement_repository::{AgreementRepository, CreateAgreementCommand},
            ledger_repository::LedgerRepository,
            openfga_port::OpenFgaPort,
        },
    },
    domain::{
        agreement::Agreement, 
        enums::{BillingFrequency, LedgerEntryType, PaymentMethodType}, 
        errors::DomainError,
        ledger::CreateLedgerEntryCommand,
    },
};

pub struct CreateAgreementUseCase {
    pub repo: Arc<dyn AgreementRepository>,
    pub ledger_repo: Arc<dyn LedgerRepository>,
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
    pub record_deposit_payment: bool,
    pub deposit_payment_method: Option<PaymentMethodType>,
    pub created_by: Uuid,
}

impl CreateAgreementUseCase {
    pub fn new(
        repo: Arc<dyn AgreementRepository>, 
        ledger_repo: Arc<dyn LedgerRepository>,
        openfga: Arc<dyn OpenFgaPort>
    ) -> Self {
        Self { repo, ledger_repo, openfga }
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

        // If deposit is positive, we might want to record it
        if input.deposit_kes > Decimal::ZERO && input.record_deposit_payment {
            // 1. Post a Deposit Charge
            self.ledger_repo.create(CreateLedgerEntryCommand {
                agreement_id: Some(agreement.id),
                unit_id: Some(input.unit_id),
                resident_id: Some(input.resident_id),
                owner_id: None,
                entry_type: LedgerEntryType::Deposit,
                amount_kes: input.deposit_kes,
                description: "Security Deposit Charge".to_string(),
                external_ref: None,
                mpesa_receipt: None,
                period_start: None,
                period_end: None,
                posted_by: input.created_by,
                metadata: serde_json::json!({}),
            }).await?;

            // 2. Post a Payment if method provided
            if let Some(method) = input.deposit_payment_method {
                let entry_type = LedgerEntryType::from_payment_method(method);
                self.ledger_repo.create(CreateLedgerEntryCommand {
                    agreement_id: Some(agreement.id),
                    unit_id: Some(input.unit_id),
                    resident_id: Some(input.resident_id),
                    owner_id: None,
                    entry_type,
                    amount_kes: input.deposit_kes, // Payments are recorded as positive amounts (credit to ledger is -ve in balance logic) Wait, balance logic does: `CASE WHEN le.entry_type IN ('payment_mpesa', ...) THEN -le.amount_kes`. So we store positive amounts for payments too.
                    description: "Security Deposit Payment".to_string(),
                    external_ref: None,
                    mpesa_receipt: None,
                    period_start: None,
                    period_end: None,
                    posted_by: input.created_by,
                    metadata: serde_json::json!({}),
                }).await?;
            }
        }

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
