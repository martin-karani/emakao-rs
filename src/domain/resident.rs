use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PortalStatus {
    Invited,
    Active,
    Suspended,
}

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
