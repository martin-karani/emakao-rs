// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added rfc3339 serialization to OffsetDateTime fields
//   - Removed submitted_by (internal audit detail)
//   - Kept reviewed_by (accounting review requirement)
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resident_id: Option<Uuid>,
    pub method_type: PaymentMethodType,
    pub amount_kes: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub status: PaymentClaimStatus,
    // reviewed_by kept: needed by frontend to display financial audit trail
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// submitted_by intentionally excluded — internal audit detail.
// Use GET /api/v1/audit-log?entity=payment_claim&id={id} for actor history.
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
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}
