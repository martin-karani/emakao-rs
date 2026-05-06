use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::payment_repository::PaymentRepository},
    domain::{
        enums::{PaymentClaimStatus, PaymentMethodType},
        payment::{CreatePaymentClaimCommand, PaymentClaim},
    },
};

pub struct PgPaymentRepo {
    pool: PgPool,
}

impl PgPaymentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgPaymentRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PaymentClaimRow {
    id: Uuid,
    property_id: Uuid,
    agreement_id: Option<Uuid>,
    resident_id: Option<Uuid>,
    method_type: String,
    amount_kes: rust_decimal::Decimal,
    reference_code: Option<String>,
    proof_url: Option<String>,
    notes: Option<String>,
    status: String,
    reviewed_by: Option<Uuid>,
    reviewed_at: Option<time::OffsetDateTime>,
    review_notes: Option<String>,
    rejection_reason: Option<String>,
    ledger_entry_id: Option<Uuid>,
    submitted_by: Uuid,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn parse_method(s: &str) -> PaymentMethodType {
    match s {
        "mpesa_paybill" => PaymentMethodType::MpesaPaybill,
        "mpesa_till" => PaymentMethodType::MpesaTill,
        "bank_transfer" => PaymentMethodType::BankTransfer,
        _ => PaymentMethodType::Cash,
    }
}

fn method_str(m: &PaymentMethodType) -> &'static str {
    match m {
        PaymentMethodType::MpesaPaybill => "mpesa_paybill",
        PaymentMethodType::MpesaTill => "mpesa_till",
        PaymentMethodType::BankTransfer => "bank_transfer",
        PaymentMethodType::Cash => "cash",
    }
}

fn parse_claim_status(s: &str) -> PaymentClaimStatus {
    match s {
        "approved" => PaymentClaimStatus::Approved,
        "rejected" => PaymentClaimStatus::Rejected,
        _ => PaymentClaimStatus::PendingReview,
    }
}

fn claim_status_str(s: &PaymentClaimStatus) -> &'static str {
    match s {
        PaymentClaimStatus::PendingReview => "pending_review",
        PaymentClaimStatus::Approved => "approved",
        PaymentClaimStatus::Rejected => "rejected",
    }
}

impl From<PaymentClaimRow> for PaymentClaim {
    fn from(r: PaymentClaimRow) -> Self {
        Self {
            id: r.id,
            property_id: r.property_id,
            agreement_id: r.agreement_id,
            resident_id: r.resident_id,
            method_type: parse_method(&r.method_type),
            amount_kes: r.amount_kes,
            reference_code: r.reference_code,
            proof_url: r.proof_url,
            notes: r.notes,
            status: parse_claim_status(&r.status),
            reviewed_by: r.reviewed_by,
            reviewed_at: r.reviewed_at,
            review_notes: r.review_notes,
            rejection_reason: r.rejection_reason,
            ledger_entry_id: r.ledger_entry_id,
            submitted_by: r.submitted_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl PaymentRepository for PgPaymentRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        status: Option<PaymentClaimStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PaymentClaim>, AppError> {
        let status_str = status.as_ref().map(claim_status_str);

        let rows = sqlx::query_as!(
            PaymentClaimRow,
            r#"
            SELECT pc.id, pc.property_id, pc.agreement_id, pc.resident_id,
                   pc.method_type, pc.amount_kes, pc.reference_code,
                   pc.proof_url, pc.notes, pc.status,
                   pc.reviewed_by, pc.reviewed_at, pc.review_notes,
                   pc.rejection_reason, pc.ledger_entry_id,
                   pc.submitted_by, pc.created_at, pc.updated_at
            FROM payment_claims pc
            JOIN properties p ON p.id = pc.property_id
            WHERE p.agency_id = $1
              AND ($2::uuid IS NULL OR pc.property_id = $2)
              AND ($3::text IS NULL OR pc.status = $3)
            ORDER BY pc.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
            agency_id,
            property_id,
            status_str,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(PaymentClaim::from).collect())
    }

    async fn find_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<PaymentClaim>, AppError> {
        let row = sqlx::query_as!(
            PaymentClaimRow,
            r#"
            SELECT pc.id, pc.property_id, pc.agreement_id, pc.resident_id,
                   pc.method_type, pc.amount_kes, pc.reference_code,
                   pc.proof_url, pc.notes, pc.status,
                   pc.reviewed_by, pc.reviewed_at, pc.review_notes,
                   pc.rejection_reason, pc.ledger_entry_id,
                   pc.submitted_by, pc.created_at, pc.updated_at
            FROM payment_claims pc
            JOIN properties p ON p.id = pc.property_id
            WHERE pc.id = $1 AND p.agency_id = $2
            "#,
            id,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(PaymentClaim::from))
    }

    async fn create(&self, cmd: CreatePaymentClaimCommand) -> Result<PaymentClaim, AppError> {
        let row = sqlx::query_as!(
            PaymentClaimRow,
            r#"
            INSERT INTO payment_claims (
                id, property_id, agreement_id, resident_id,
                method_type, amount_kes, reference_code, proof_url,
                notes, status, submitted_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending_review', $10)
            RETURNING
                id, property_id, agreement_id, resident_id,
                method_type, amount_kes, reference_code, proof_url, notes, status,
                reviewed_by, reviewed_at, review_notes, rejection_reason,
                ledger_entry_id, submitted_by, created_at, updated_at
            "#,
            Uuid::new_v4(),
            cmd.property_id,
            cmd.agreement_id,
            cmd.resident_id,
            method_str(&cmd.method_type),
            cmd.amount_kes,
            cmd.reference_code,
            cmd.proof_url,
            cmd.notes,
            cmd.submitted_by
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(PaymentClaim::from(row))
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: PaymentClaimStatus,
        reviewed_by: Uuid,
        review_notes: Option<String>,
        rejection_reason: Option<String>,
    ) -> Result<PaymentClaim, AppError> {
        let row = sqlx::query_as!(
            PaymentClaimRow,
            r#"
            UPDATE payment_claims
            SET
                status           = $2,
                reviewed_by      = $3,
                reviewed_at      = now(),
                review_notes     = $4,
                rejection_reason = $5,
                updated_at       = now()
            WHERE id = $1
            RETURNING
                id, property_id, agreement_id, resident_id,
                method_type, amount_kes, reference_code, proof_url, notes, status,
                reviewed_by, reviewed_at, review_notes, rejection_reason,
                ledger_entry_id, submitted_by, created_at, updated_at
            "#,
            id,
            claim_status_str(&status),
            reviewed_by,
            review_notes,
            rejection_reason
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("payment claim {id}")))?;

        Ok(PaymentClaim::from(row))
    }
}
