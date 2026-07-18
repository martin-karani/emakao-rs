use std::sync::Arc;

use rust_decimal::Decimal;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::property_expense_repository::PropertyExpenseRepository},
    domain::property_expense::{
        CreatePropertyExpenseCommand, PropertyExpense, PropertyExpenseCategory,
    },
};

pub struct CreatePropertyExpenseUseCase {
    pub repo: Arc<dyn PropertyExpenseRepository>,
}

pub struct CreatePropertyExpenseInput {
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub expense_date: Date,
    pub category: PropertyExpenseCategory,
    pub description: String,
    pub vendor_name: Option<String>,
    pub amount_kes: Decimal,
    pub notes: Option<String>,
    pub created_by: Uuid,
}

impl CreatePropertyExpenseUseCase {
    pub fn new(repo: Arc<dyn PropertyExpenseRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        input: CreatePropertyExpenseInput,
    ) -> Result<PropertyExpense, AppError> {
        let description = input.description.trim();
        if description.is_empty() {
            return Err(AppError::Validation("description is required".into()));
        }

        if input.amount_kes <= Decimal::ZERO {
            return Err(AppError::Validation(
                "amount_kes must be greater than zero".into(),
            ));
        }

        let vendor_name = input
            .vendor_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);

        let notes = input
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);

        self.repo
            .create(CreatePropertyExpenseCommand {
                agency_id: input.agency_id,
                property_id: input.property_id,
                expense_date: input.expense_date,
                category: input.category,
                description: description.to_owned(),
                vendor_name,
                amount_kes: input.amount_kes,
                notes,
                created_by: input.created_by,
            })
            .await
    }
}
