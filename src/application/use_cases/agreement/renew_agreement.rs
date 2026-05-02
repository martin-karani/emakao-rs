use std::sync::Arc;
use uuid::Uuid;
use rust_decimal::Decimal;
use time::Date;
use crate::{
    application::{
        errors::AppError,
        ports::agreement_repository::{AgreementRepository, UpdateAgreementCommand},
    },
    domain::{agreement::{Agreement, AgreementStatus}, errors::DomainError},
};

pub struct RenewAgreementUseCase { pub repo: Arc<dyn AgreementRepository> }

impl RenewAgreementUseCase {
    pub fn new(repo: Arc<dyn AgreementRepository>) -> Self { Self { repo } }

    pub async fn execute(
        &self,
        id: Uuid,
        new_end_date: Option<Date>,
        new_rent: Option<Decimal>,
    ) -> Result<Agreement, AppError> {
        let existing = self.repo.find_by_id(id).await?
            .ok_or_else(|| AppError::NotFound(format!("agreement {id}")))?;

        if existing.status == AgreementStatus::Terminated {
            return Err(DomainError::AgreementNotActive(id).into());
        }

        let cmd = UpdateAgreementCommand {
            id,
            end_date: new_end_date,
            rent_amount_kes: new_rent,
            billing_frequency: None,
            status: Some(AgreementStatus::Active),
        };

        self.repo.update(cmd).await
    }
}
