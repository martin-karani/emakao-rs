use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{
    enums::{PaymentClaimStatus, PaymentMethodType},
    payment::PaymentClaim,
};
#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentClaimResponse {
    pub id: Uuid,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub method_type: PaymentMethodType,
    pub amount_kes: Decimal,
    pub reference_code: Option<String>,
    pub proof_url: Option<String>,
    pub notes: Option<String>,
    pub status: PaymentClaimStatus,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<OffsetDateTime>,
    pub review_notes: Option<String>,
    pub rejection_reason: Option<String>,
    pub submitted_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<PaymentClaim> for PaymentClaimResponse {
    fn from(c: PaymentClaim) -> Self {
        Self {
            id: c.id,
            property_id: c.property_id,
            agreement_id: c.agreement_id,
            resident_id: c.resident_id,
            method_type: c.method_type,
            amount_kes: c.amount_kes,
            reference_code: c.reference_code,
            proof_url: c.proof_url,
            notes: c.notes,
            status: c.status,
            reviewed_by: c.reviewed_by,
            reviewed_at: c.reviewed_at,
            review_notes: c.review_notes,
            rejection_reason: c.rejection_reason,
            submitted_by: c.submitted_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}
