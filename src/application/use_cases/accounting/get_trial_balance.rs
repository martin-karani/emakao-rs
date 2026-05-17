use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::accounting_repository::AccountingRepository},
    domain::accounting::TrialBalance,
};

pub struct GetTrialBalanceUseCase {
    pub repo: Arc<dyn AccountingRepository>,
}

impl GetTrialBalanceUseCase {
    pub async fn execute(&self, agency_id: Uuid) -> Result<TrialBalance, AppError> {
        self.repo.get_trial_balance(agency_id).await
    }
}
