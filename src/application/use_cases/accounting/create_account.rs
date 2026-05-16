// src/application/use_cases/accounting/create_account.rs

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::accounting_repository::AccountingRepository},
    domain::accounting::{Account, AccountType, CreateAccountCommand},
};

pub struct CreateAccountInput {
    pub agency_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
}

pub struct CreateAccountUseCase {
    pub repo: Arc<dyn AccountingRepository>,
}

impl CreateAccountUseCase {
    pub async fn execute(&self, input: CreateAccountInput) -> Result<Account, AppError> {
        if input.code.trim().is_empty() {
            return Err(AppError::Validation("account code is required".into()));
        }
        if input.name.trim().is_empty() {
            return Err(AppError::Validation("account name is required".into()));
        }

        self.repo
            .create_account(CreateAccountCommand {
                agency_id: input.agency_id,
                code: input.code.trim().to_string(),
                name: input.name.trim().to_string(),
                account_type: input.account_type,
                is_system: false, // only provisioning code sets this to true
            })
            .await
    }
}
