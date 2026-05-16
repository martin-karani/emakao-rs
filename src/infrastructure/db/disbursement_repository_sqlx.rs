// src/infrastructure/db/disbursement_repository_sqlx.rs

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::disbursement_repository::{DisbursementFilter, DisbursementRepository},
    },
    domain::{
        disbursement::{CreateDisbursementCommand, Disbursement},
        enums::{DisbursementMethod, DisbursementStatus},
    },
};

pub struct PgDisbursementRepo {
    pool: PgPool,
}

impl PgDisbursementRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Row type ─────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct DisbursementRow {
    id: Uuid,
    agency_id: Uuid,
    owner_id: Uuid,
    property_id: Uuid,
    amount_kes: Decimal,
    method: String,
    reference: Option<String>,
    status: String,
    period_start: time::Date,
    period_end: time::Date,
    notes: Option<String>,
    created_by: Uuid,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn parse_method(s: &str) -> DisbursementMethod {
    match s {
        "bank_transfer" => DisbursementMethod::BankTransfer,
        "mpesa_b2c" => DisbursementMethod::MpesaB2c,
        _ => DisbursementMethod::Cheque,
    }
}

fn parse_status(s: &str) -> DisbursementStatus {
    match s {
        "processing" => DisbursementStatus::Processing,
        "completed" => DisbursementStatus::Completed,
        "failed" => DisbursementStatus::Failed,
        _ => DisbursementStatus::Pending,
    }
}

fn method_str(m: &DisbursementMethod) -> &'static str {
    match m {
        DisbursementMethod::BankTransfer => "bank_transfer",
        DisbursementMethod::MpesaB2c => "mpesa_b2c",
        DisbursementMethod::Cheque => "cheque",
    }
}

fn status_str(s: &DisbursementStatus) -> &'static str {
    match s {
        DisbursementStatus::Pending => "pending",
        DisbursementStatus::Processing => "processing",
        DisbursementStatus::Completed => "completed",
        DisbursementStatus::Failed => "failed",
    }
}

impl From<DisbursementRow> for Disbursement {
    fn from(r: DisbursementRow) -> Self {
        Self {
            id: r.id,
            agency_id: r.agency_id,
            owner_id: r.owner_id,
            property_id: r.property_id,
            amount_kes: r.amount_kes,
            method: parse_method(&r.method),
            reference: r.reference,
            status: parse_status(&r.status),
            period_start: r.period_start,
            period_end: r.period_end,
            notes: r.notes,
            created_by: r.created_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ── SELECT fragment ───────────────────────────────────────────────────────────

const COLS: &str = r#"
    id, agency_id, owner_id, property_id, amount_kes,
    method::text, reference, status::text,
    period_start, period_end, notes,
    created_by, created_at, updated_at
"#;

// ── Repository impl ───────────────────────────────────────────────────────────

#[async_trait]
impl DisbursementRepository for PgDisbursementRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        filter: DisbursementFilter,
    ) -> Result<Vec<Disbursement>, AppError> {
        let rows = sqlx::query_as!(
            DisbursementRow,
            r#"
            SELECT
                id, agency_id, owner_id, property_id, amount_kes,
                method::text AS "method!", reference, status::text AS "status!",
                period_start, period_end, notes,
                created_by, created_at, updated_at
            FROM disbursements
            WHERE agency_id  = $1
              AND ($2::uuid IS NULL OR owner_id    = $2)
              AND ($3::uuid IS NULL OR property_id = $3)
              AND ($4::text IS NULL OR status::text = $4)
            ORDER BY created_at DESC
            LIMIT $5 OFFSET $6
            "#,
            agency_id,
            filter.owner_id,
            filter.property_id,
            filter.status.as_ref().map(status_str),
            filter.limit,
            filter.offset,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Disbursement::from).collect())
    }

    async fn find_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Disbursement>, AppError> {
        let row = sqlx::query_as!(
            DisbursementRow,
            r#"
            SELECT
                id, agency_id, owner_id, property_id, amount_kes,
                method::text AS "method!", reference, status::text AS "status!",
                period_start, period_end, notes,
                created_by, created_at, updated_at
            FROM disbursements
            WHERE id = $1 AND agency_id = $2
            "#,
            id,
            agency_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Disbursement::from))
    }

    async fn create(&self, cmd: CreateDisbursementCommand) -> Result<Disbursement, AppError> {
        let row = sqlx::query_as!(
            DisbursementRow,
            r#"
            INSERT INTO disbursements (
                agency_id, owner_id, property_id,
                amount_kes, method, period_start, period_end,
                notes, created_by
            )
            VALUES (
                $1, $2, $3,
                $4, $5::disbursement_method, $6::date, $7::date,
                $8, $9
            )
            RETURNING
                id, agency_id, owner_id, property_id, amount_kes,
                method::text AS "method!", reference, status::text AS "status!",
                period_start, period_end, notes,
                created_by, created_at, updated_at
            "#,
            cmd.agency_id,
            cmd.owner_id,
            cmd.property_id,
            cmd.amount_kes,
            method_str(&cmd.method),
            cmd.period_start,
            cmd.period_end,
            cmd.notes,
            cmd.created_by,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Disbursement::from(row))
    }

    async fn update_status(
        &self,
        agency_id: Uuid,
        id: Uuid,
        status: DisbursementStatus,
        reference: Option<String>,
    ) -> Result<Disbursement, AppError> {
        let row = sqlx::query_as!(
            DisbursementRow,
            r#"
            UPDATE disbursements
            SET status    = $3::disbursement_status,
                reference = COALESCE($4, reference),
                updated_at = now()
            WHERE id = $1 AND agency_id = $2
            RETURNING
                id, agency_id, owner_id, property_id, amount_kes,
                method::text AS "method!", reference, status::text AS "status!",
                period_start, period_end, notes,
                created_by, created_at, updated_at
            "#,
            id,
            agency_id,
            status_str(&status),
            reference,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("disbursement {id}")))?;

        Ok(Disbursement::from(row))
    }
}
