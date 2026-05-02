use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationStatus {
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Withdrawn,
}

#[derive(Clone, Debug, Serialize)]
pub struct Applicant {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub national_id: Option<String>,
    pub monthly_income_kes: Option<rust_decimal::Decimal>,
    pub employer: Option<String>,
    pub status: ApplicationStatus,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
