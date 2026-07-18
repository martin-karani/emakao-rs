use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::property_expense::PropertyExpense;

#[derive(Debug, Serialize, ToSchema)]
pub struct PropertyExpenseResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub expense_date: Date,
    pub category: crate::domain::property_expense::PropertyExpenseCategory,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_name: Option<String>,
    pub amount_kes: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_by: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<PropertyExpense> for PropertyExpenseResponse {
    fn from(expense: PropertyExpense) -> Self {
        Self {
            id: expense.id,
            agency_id: expense.agency_id,
            property_id: expense.property_id,
            expense_date: expense.expense_date,
            category: expense.category,
            description: expense.description,
            vendor_name: expense.vendor_name,
            amount_kes: expense.amount_kes,
            notes: expense.notes,
            created_by: expense.created_by,
            created_at: expense.created_at,
            updated_at: expense.updated_at,
        }
    }
}
