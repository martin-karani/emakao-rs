use crate::domain::enums::PortalStatus;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resident {
    pub id: Uuid,
    /// None until the invite is accepted and the user sets a password.
    pub user_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: String,
    /// Primary email — may be None for phone-only residents.
    pub email: Option<String>,
    pub phone: Option<String>,
    pub national_id: Option<String>,
    pub portal_status: PortalStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Resident {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// The contact value that was used for invitation.
    pub fn primary_contact(&self) -> Option<&str> {
        self.email.as_deref().or(self.phone.as_deref())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantWithLease {
    pub resident_id: Uuid,
    pub resident_name: String,
    pub resident_email: Option<String>,
    pub resident_phone: Option<String>,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub agreement_id: Uuid,
    pub rent_amount_kes: rust_decimal::Decimal,
    pub deposit_kes: rust_decimal::Decimal,
    pub status: crate::domain::enums::AgreementStatus,
    pub outstanding_balance: rust_decimal::Decimal,
    pub deposit_paid: rust_decimal::Decimal,
    pub start_date: time::Date,
    pub end_date: Option<time::Date>,
}
