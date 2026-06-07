use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};

use crate::domain::accounting::{AccountType, JournalLine};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListAccountsParams {
    pub account_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListJournalEntriesParams {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct VatReportParams {
    pub period_start: Date,
    pub period_end: Date,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountDto {
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    #[serde(default)]
    pub vat_applicable: bool,
    pub vat_rate: Option<Decimal>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PostJournalEntryDto {
    pub reference: String,
    pub description: Option<String>,
    pub lines: Vec<JournalLine>,
    #[serde(default = "bool_true")]
    pub post_immediately: bool,
}

fn default_limit() -> i64 {
    50
}

fn bool_true() -> bool {
    true
}
