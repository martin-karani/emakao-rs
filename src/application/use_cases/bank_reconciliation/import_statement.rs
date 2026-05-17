use std::sync::Arc;

use rust_decimal::Decimal;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::bank_reconciliation_repository::BankReconciliationRepository,
    },
    domain::bank_reconciliation::{
        BankStatement, ImportStatementCommand, ImportStatementLineInput,
    },
};

pub struct ImportStatementLineInput2 {
    pub value_date: Date,
    pub description: String,
    pub amount: Decimal,
    pub reference: Option<String>,
}

pub struct ImportStatementInput {
    pub agency_id: Uuid,
    pub bank_name: String,
    pub account_number: String,
    pub statement_date: Date,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub lines: Vec<ImportStatementLineInput2>,
    pub created_by: Uuid,
}

pub struct ImportStatementUseCase {
    pub repo: Arc<dyn BankReconciliationRepository>,
}

impl ImportStatementUseCase {
    pub async fn execute(&self, input: ImportStatementInput) -> Result<BankStatement, AppError> {
        if input.lines.is_empty() {
            return Err(AppError::Validation(
                "a bank statement must have at least one line".into(),
            ));
        }

        if input.bank_name.trim().is_empty() || input.account_number.trim().is_empty() {
            return Err(AppError::Validation(
                "bank_name and account_number are required".into(),
            ));
        }

        self.repo
            .import_statement(ImportStatementCommand {
                agency_id: input.agency_id,
                bank_name: input.bank_name.trim().to_string(),
                account_number: input.account_number.trim().to_string(),
                statement_date: input.statement_date,
                opening_balance: input.opening_balance,
                closing_balance: input.closing_balance,
                lines: input
                    .lines
                    .into_iter()
                    .map(|l| ImportStatementLineInput {
                        value_date: l.value_date,
                        description: l.description.trim().to_string(),
                        amount: l.amount,
                        reference: l.reference,
                    })
                    .collect(),
                created_by: input.created_by,
            })
            .await
    }
}
