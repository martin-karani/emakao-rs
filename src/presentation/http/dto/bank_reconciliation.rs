use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListStatementsParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportStatementLineDto {
    pub value_date: Date,
    pub description: String,
    pub amount: Decimal,
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportStatementDto {
    pub bank_name: String,
    pub account_number: String,
    pub statement_date: Date,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub lines: Vec<ImportStatementLineDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MatchLineDto {
    pub journal_entry_id: Uuid,
}

fn default_limit() -> i64 {
    20
}
