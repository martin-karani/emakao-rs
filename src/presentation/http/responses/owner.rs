use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::owner::{Owner, OwnerPortalStatus};

#[derive(Debug, Serialize)]
pub struct OwnerResponse {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub display_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
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

impl From<Owner> for OwnerResponse {
    fn from(o: Owner) -> Self {
        let display_name = o.display_name();
        Self {
            id: o.id,
            user_id: o.user_id,
            display_name,
            first_name: o.first_name,
            last_name: o.last_name,
            email: o.email,
            phone: o.phone,
            company_name: o.company_name,
            kra_pin: o.kra_pin,
            bank_name: o.bank_name,
            bank_account: o.bank_account,
            mpesa_number: o.mpesa_number,
            portal_status: o.portal_status,
            created_at: o.created_at,
            updated_at: o.updated_at,
        }
    }
}
