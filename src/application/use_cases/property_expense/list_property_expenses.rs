use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::property_expense_repository::PropertyExpenseRepository},
    domain::property_expense::PropertyExpense,
};

pub struct ListPropertyExpensesUseCase {
    pub repo: Arc<dyn PropertyExpenseRepository>,
}

impl ListPropertyExpensesUseCase {
    pub fn new(repo: Arc<dyn PropertyExpenseRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        property_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PropertyExpense>, AppError> {
        self.repo
            .list_for_property(agency_id, property_id, limit, offset)
            .await
    }
}
