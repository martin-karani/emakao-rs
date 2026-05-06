use crate::domain::enums::{PaymentClaimStatus, PaymentMethodType};
use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct PaymentClaim {
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
    pub ledger_entry_id: Option<Uuid>,
    pub submitted_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Command passed to `PaymentRepository::create`.
pub struct CreatePaymentClaimCommand {
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub submitted_by: Uuid,
    pub method_type: PaymentMethodType,
    pub amount_kes: Decimal,
    pub reference_code: Option<String>,
    pub proof_url: Option<String>,
    pub notes: Option<String>,
}
