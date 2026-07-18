use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PropertyExpenseCategory {
    Maintenance,
    Security,
    Utilities,
    Payroll,
    Management,
    Taxes,
    Insurance,
    Supplies,
    BankCharges,
    Other,
}

impl PropertyExpenseCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Maintenance => "maintenance",
            Self::Security => "security",
            Self::Utilities => "utilities",
            Self::Payroll => "payroll",
            Self::Management => "management",
            Self::Taxes => "taxes",
            Self::Insurance => "insurance",
            Self::Supplies => "supplies",
            Self::BankCharges => "bank_charges",
            Self::Other => "other",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "maintenance" => Some(Self::Maintenance),
            "security" => Some(Self::Security),
            "utilities" => Some(Self::Utilities),
            "payroll" => Some(Self::Payroll),
            "management" => Some(Self::Management),
            "taxes" => Some(Self::Taxes),
            "insurance" => Some(Self::Insurance),
            "supplies" => Some(Self::Supplies),
            "bank_charges" => Some(Self::BankCharges),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PropertyExpense {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub expense_date: Date,
    pub category: PropertyExpenseCategory,
    pub description: String,
    pub vendor_name: Option<String>,
    pub amount_kes: Decimal,
    pub notes: Option<String>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct CreatePropertyExpenseCommand {
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
