use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::disbursement::Disbursement;
use crate::domain::enums::{DisbursementMethod, DisbursementStatus, PortalStatus};
use crate::domain::owner::Owner;
#[derive(Debug, Serialize, ToSchema)]
pub struct OwnerResponse {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub display_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub kra_pin: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub mpesa_number: Option<String>,
    pub portal_status: PortalStatus,
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

#[derive(Debug, Serialize, ToSchema)]
pub struct DisbursementResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub owner_id: Uuid,
    pub property_id: Uuid,
    pub amount_kes: Decimal,
    pub method: DisbursementMethod,
    pub reference: Option<String>,
    pub status: DisbursementStatus,
    pub period_start: time::Date,
    pub period_end: time::Date,
    pub notes: Option<String>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Disbursement> for DisbursementResponse {
    fn from(d: Disbursement) -> Self {
        Self {
            id: d.id,
            agency_id: d.agency_id,
            owner_id: d.owner_id,
            property_id: d.property_id,
            amount_kes: d.amount_kes,
            method: d.method,
            reference: d.reference,
            status: d.status,
            period_start: d.period_start,
            period_end: d.period_end,
            notes: d.notes,
            created_by: d.created_by,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}
