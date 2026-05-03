use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OwnerPortalStatus {
    Invited,
    Active,
    Suspended,
}

#[derive(Clone, Debug, Serialize)]
pub struct Owner {
    pub id: Uuid,
    /// None until invite is accepted.
    pub user_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: String,
    /// May be None for phone-only owners.
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub kra_pin: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub mpesa_number: Option<String>,
    pub portal_status: OwnerPortalStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Owner {
    pub fn display_name(&self) -> String {
        self.company_name
            .clone()
            .unwrap_or_else(|| format!("{} {}", self.first_name, self.last_name))
    }

    pub fn primary_contact(&self) -> Option<&str> {
        self.email.as_deref().or(self.phone.as_deref())
    }
}

pub struct CreateOwnerCommand {
    pub user_id: Option<Uuid>,
    pub agency_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub kra_pin: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub mpesa_number: Option<String>,
}

pub struct UpdateOwnerCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub kra_pin: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub mpesa_number: Option<String>,
}
