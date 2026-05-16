use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::accounting_repository::AccountingRepository},
    domain::accounting::{Account, AccountFilter, AccountType},
};

pub struct ListAccountsInput {
    pub agency_id: Uuid,
    pub account_type: Option<AccountType>,
    pub limit: i64,
    pub offset: i64,
}

pub struct ListAccountsUseCase {
    pub repo: Arc<dyn AccountingRepository>,
}

impl ListAccountsUseCase {
    pub async fn execute(&self, input: ListAccountsInput) -> Result<Vec<Account>, AppError> {
        self.repo
            .list_accounts(AccountFilter {
                agency_id: input.agency_id,
                account_type: input.account_type,
                limit: input.limit.clamp(1, 200),
                offset: input.offset.max(0),
            })
            .await
    }
}
