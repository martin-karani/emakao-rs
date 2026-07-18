use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};

use crate::domain::property_expense::PropertyExpenseCategory;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePropertyExpenseDto {
    #[garde(length(min = 1, max = 500))]
    pub description: String,

    #[garde(skip)]
    pub category: PropertyExpenseCategory,

    #[garde(skip)]
    pub amount_kes: Decimal,

    #[garde(skip)]
    pub expense_date: Date,

    #[garde(length(max = 200))]
    pub vendor_name: Option<String>,

    #[garde(length(max = 4000))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListPropertyExpensesParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
