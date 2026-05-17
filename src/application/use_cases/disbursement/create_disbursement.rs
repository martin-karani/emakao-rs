use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::disbursement_repository::DisbursementRepository},
    domain::{
        disbursement::{CreateDisbursementCommand, Disbursement},
        enums::DisbursementMethod,
    },
};

pub struct CreateDisbursementInput {
    pub agency_id: Uuid,
    pub owner_id: Uuid,
    pub property_id: Uuid,
    pub amount_kes: rust_decimal::Decimal,
    pub method: DisbursementMethod,
    pub period_start: time::Date,
    pub period_end: time::Date,
    pub notes: Option<String>,
    pub created_by: Uuid,
}

pub struct CreateDisbursementUseCase {
    repo: Arc<dyn DisbursementRepository>,
}

impl CreateDisbursementUseCase {
    pub fn new(repo: Arc<dyn DisbursementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateDisbursementInput) -> Result<Disbursement, AppError> {
        if input.amount_kes <= rust_decimal::Decimal::ZERO {
            return Err(AppError::Validation(
                "Disbursement amount must be greater than zero".into(),
            ));
        }
        if input.period_end < input.period_start {
            return Err(AppError::Validation(
                "period_end must be on or after period_start".into(),
            ));
        }

        self.repo
            .create(CreateDisbursementCommand {
                agency_id: input.agency_id,
                owner_id: input.owner_id,
                property_id: input.property_id,
                amount_kes: input.amount_kes,
                method: input.method,
                period_start: input.period_start,
                period_end: input.period_end,
                notes: input.notes,
                created_by: input.created_by,
            })
            .await
    }
}
