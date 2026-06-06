use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::resident_repository::{CreateResidentCommand, ResidentRepository},
    },
    domain::{
        enums::{PaymentClaimStatus, PaymentMethodType, PortalStatus},
        payment::PaymentClaim,
        resident::Resident,
    },
};

pub struct PgResidentRepo {
    pool: PgPool,
}

impl PgResidentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgResidentRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ResidentRow {
    id: Uuid,
    user_id: Option<Uuid>,
    first_name: String,
    last_name: String,
    email: Option<String>,
    phone: Option<String>,
    national_id: Option<String>,
    portal_status: String,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn parse_portal_status(s: &str) -> PortalStatus {
    match s {
        "active" => PortalStatus::Active,
        "suspended" => PortalStatus::Suspended,
        _ => PortalStatus::Invited,
    }
}

impl From<ResidentRow> for Resident {
    fn from(r: ResidentRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            first_name: r.first_name,
            last_name: r.last_name,
            email: r.email,
            phone: r.phone,
            national_id: r.national_id,
            portal_status: parse_portal_status(&r.portal_status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl ResidentRepository for PgResidentRepo {
    async fn find_all(
        &self,
        _agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Resident>, AppError> {
        let rows = sqlx::query_as::<_, ResidentRow>(
            r#"
            SELECT r.id, r.user_id, r.first_name, r.last_name, r.email,
                   r.phone, r.national_id, r.portal_status::text, r.created_at, r.updated_at
            FROM residents r
            ORDER BY r.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(Resident::from).collect())
    }

    async fn find_by_id(&self, _agency_id: Uuid, id: Uuid) -> Result<Option<Resident>, AppError> {
        let row = sqlx::query_as::<_, ResidentRow>(
            r#"
            SELECT r.id, r.user_id, r.first_name, r.last_name, r.email,
                   r.phone, r.national_id, r.portal_status::text, r.created_at, r.updated_at
            FROM residents r
            WHERE r.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(Resident::from))
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Resident>, AppError> {
        let row = sqlx::query_as::<_, ResidentRow>(
            r#"
            SELECT r.id, r.user_id, r.first_name, r.last_name, r.email,
                   r.phone, r.national_id, r.portal_status::text, r.created_at, r.updated_at
            FROM residents r
            WHERE r.user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(Resident::from))
    }

    async fn find_by_email(
        &self,
        _agency_id: Uuid,
        email: &str,
    ) -> Result<Option<Resident>, AppError> {
        let row = sqlx::query_as::<_, ResidentRow>(
            r#"
            SELECT r.id, r.user_id, r.first_name, r.last_name, r.email,
                   r.phone, r.national_id, r.portal_status::text, r.created_at, r.updated_at
            FROM residents r
            WHERE r.email = $1
            LIMIT 1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(Resident::from))
    }

    async fn create(&self, cmd: CreateResidentCommand) -> Result<Resident, AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let row = sqlx::query_as::<_, ResidentRow>(
            r#"
            INSERT INTO residents (
                id, user_id, first_name, last_name, email,
                phone, national_id, portal_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::portal_status)
            RETURNING
                id, user_id, first_name, last_name, email,
                phone, national_id, portal_status::text, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(cmd.user_id)
        .bind(cmd.first_name)
        .bind(cmd.last_name)
        .bind(cmd.email)
        .bind(cmd.phone)
        .bind(cmd.national_id)
        .bind("invited")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(Resident::from(row))
    }

    async fn find_payment_claims_by_resident_id(
        &self,
        resident_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PaymentClaim>, AppError> {
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

        let rows = sqlx::query_as::<_, PaymentClaimRow>(
            r#"
        SELECT id, property_id, agreement_id, resident_id,
               method_type::text, amount_kes, reference_code, proof_url,
               notes, status::text, reviewed_by, reviewed_at,
               review_notes, rejection_reason, ledger_entry_id,
               submitted_by, created_at, updated_at
        FROM payment_claims
        WHERE resident_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        )
        .bind(resident_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        fn parse_method(s: &str) -> PaymentMethodType {
            match s {
                "mpesa_paybill" => PaymentMethodType::MpesaPaybill,
                "mpesa_till" => PaymentMethodType::MpesaTill,
                "bank_transfer" => PaymentMethodType::BankTransfer,
                _ => PaymentMethodType::Cash,
            }
        }

        fn parse_status(s: &str) -> PaymentClaimStatus {
            match s {
                "approved" => PaymentClaimStatus::Approved,
                "rejected" => PaymentClaimStatus::Rejected,
                _ => PaymentClaimStatus::PendingReview,
            }
        }

        Ok(rows
            .into_iter()
            .map(|r| PaymentClaim {
                id: r.id,
                property_id: r.property_id,
                agreement_id: r.agreement_id,
                resident_id: r.resident_id,
                method_type: parse_method(&r.method_type),
                amount_kes: r.amount_kes,
                reference_code: r.reference_code,
                proof_url: r.proof_url,
                notes: r.notes,
                status: parse_status(&r.status),
                reviewed_by: r.reviewed_by,
                reviewed_at: r.reviewed_at,
                review_notes: r.review_notes,
                rejection_reason: r.rejection_reason,
                ledger_entry_id: r.ledger_entry_id,
                submitted_by: r.submitted_by,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    async fn find_by_property_id(
        &self,
        _agency_id: Uuid,
        property_id: Uuid,
    ) -> Result<Vec<Resident>, AppError> {
        let rows = sqlx::query_as::<_, ResidentRow>(
            r#"
            SELECT DISTINCT r.id, r.user_id, r.first_name, r.last_name, r.email,
                   r.phone, r.national_id, r.portal_status::text, r.created_at, r.updated_at
            FROM residents r
            JOIN agreements a ON a.resident_id = r.id
            WHERE a.property_id = $1 AND a.status = 'active'
            "#,
        )
        .bind(property_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(Resident::from).collect())
    }
}
