use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::property_expense::{CreatePropertyExpenseCommand, PropertyExpense},
};

#[async_trait]
pub trait PropertyExpenseRepository: Send + Sync + 'static {
    async fn list_for_property(
        &self,
        agency_id: Uuid,
        property_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PropertyExpense>, AppError>;

    async fn create(
        &self,
        command: CreatePropertyExpenseCommand,
    ) -> Result<PropertyExpense, AppError>;
}
