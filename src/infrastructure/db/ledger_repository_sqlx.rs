// src/infrastructure/db/ledger_repository_sqlx.rs

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::ledger_repository::LedgerRepository},
    domain::{
        enums::LedgerEntryType,
        ledger::{BalanceSummary, CreateLedgerEntryCommand, LedgerEntry},
    },
};

pub struct PgLedgerRepo {
    pool: PgPool,
}

impl PgLedgerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgLedgerRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}
pub trait HasPool {
    fn pool(&self) -> &sqlx::PgPool;
}

impl HasPool for PgLedgerRepo {
    fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}
#[derive(sqlx::FromRow)]
struct LedgerEntryRow {
    id: Uuid,
    agreement_id: Option<Uuid>,
    unit_id: Option<Uuid>,
    resident_id: Option<Uuid>,
    owner_id: Option<Uuid>,
    entry_type: LedgerEntryType, // Rust enum
    amount_kes: Decimal,
    description: String,
    external_ref: Option<String>,
    mpesa_receipt: Option<String>,
    period_start: Option<time::Date>,
    period_end: Option<time::Date>,
    posted_by: Uuid,
    posted_at: time::OffsetDateTime,
    is_reconciled: bool,
    metadata: serde_json::Value,
}

#[async_trait]
impl LedgerRepository for PgLedgerRepo {
    async fn create(&self, cmd: CreateLedgerEntryCommand) -> Result<LedgerEntry, AppError> {
        // Insert with explicit casting of the entry_type (the Rust enum is automatically
        // mapped to the DB enum because of `#[sqlx(type_name = "ledger_entry_type")]`).
        let row = sqlx::query_as!(
            LedgerEntryRow,
            r#"
                INSERT INTO ledger_entries (
                    id, agreement_id, unit_id, resident_id, owner_id,
                    entry_type, amount_kes, description, external_ref,
                    mpesa_receipt, period_start, period_end, posted_by, metadata
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                RETURNING
                    id, agreement_id, unit_id, resident_id, owner_id,
                    entry_type AS "entry_type: LedgerEntryType",
                    amount_kes, description, external_ref,
                    mpesa_receipt, period_start, period_end, posted_by,
                    posted_at, is_reconciled,
                    metadata AS "metadata: serde_json::Value"
                "#,
            Uuid::new_v4(),
            cmd.agreement_id,
            cmd.unit_id,
            cmd.resident_id,
            cmd.owner_id,
            cmd.entry_type as LedgerEntryType, // use the enum directly
            cmd.amount_kes,
            cmd.description,
            cmd.external_ref,
            cmd.mpesa_receipt,
            cmd.period_start,
            cmd.period_end,
            cmd.posted_by,
            sqlx::types::Json(cmd.metadata),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(LedgerEntry {
            id: row.id,
            agreement_id: row.agreement_id,
            unit_id: row.unit_id,
            resident_id: row.resident_id,
            owner_id: row.owner_id,
            entry_type: row.entry_type,
            amount_kes: row.amount_kes,
            description: row.description,
            external_ref: row.external_ref,
            mpesa_receipt: row.mpesa_receipt,
            period_start: row.period_start,
            period_end: row.period_end,
            posted_by: row.posted_by,
            posted_at: row.posted_at,
            is_reconciled: row.is_reconciled,
            metadata: row.metadata,
        })
    }

    async fn find_by_agreement(
        &self,
        agreement_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LedgerEntry>, AppError> {
        let rows = sqlx::query_as!(
            LedgerEntryRow,
            r#"
                SELECT
                    id, agreement_id, unit_id, resident_id, owner_id,
                    entry_type AS "entry_type: LedgerEntryType",
                    amount_kes, description, external_ref,
                    mpesa_receipt, period_start, period_end, posted_by,
                    posted_at, is_reconciled,
                    metadata AS "metadata: serde_json::Value"
                FROM ledger_entries
                WHERE agreement_id = $1
                ORDER BY posted_at DESC
                LIMIT $2 OFFSET $3
                "#,
            agreement_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| LedgerEntry {
                id: row.id,
                agreement_id: row.agreement_id,
                unit_id: row.unit_id,
                resident_id: row.resident_id,
                owner_id: row.owner_id,
                entry_type: row.entry_type,
                amount_kes: row.amount_kes,
                description: row.description,
                external_ref: row.external_ref,
                mpesa_receipt: row.mpesa_receipt,
                period_start: row.period_start,
                period_end: row.period_end,
                posted_by: row.posted_by,
                posted_at: row.posted_at,
                is_reconciled: row.is_reconciled,
                metadata: row.metadata,
            })
            .collect())
    }

    async fn balance_for_agreement(&self, agreement_id: Uuid) -> Result<BalanceSummary, AppError> {
        // Use the correct enum strings directly in the FILTER clause.
        let row = sqlx::query!(
            r#"
                SELECT
                    COALESCE(SUM(amount_kes) FILTER (
                        WHERE entry_type NOT IN (
                            'payment_mpesa'::ledger_entry_type,
                            'payment_bank'::ledger_entry_type,
                            'payment_cash'::ledger_entry_type,
                            'credit_note'::ledger_entry_type,
                            'waiver'::ledger_entry_type,
                            'deposit_refund'::ledger_entry_type
                        )
                    ), 0)  AS "total_charged!: Decimal",

                    COALESCE(SUM(amount_kes) FILTER (
                        WHERE entry_type IN (
                            'payment_mpesa'::ledger_entry_type,
                            'payment_bank'::ledger_entry_type,
                            'payment_cash'::ledger_entry_type,
                            'credit_note'::ledger_entry_type,
                            'waiver'::ledger_entry_type,
                            'deposit_refund'::ledger_entry_type
                        )
                    ), 0)  AS "total_paid!: Decimal",

                    MAX(posted_at) FILTER (
                        WHERE entry_type IN (
                            'payment_mpesa'::ledger_entry_type,
                            'payment_bank'::ledger_entry_type,
                            'payment_cash'::ledger_entry_type
                        )
                    ) AS last_payment_at

                FROM ledger_entries
                WHERE agreement_id = $1
                "#,
            agreement_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(BalanceSummary {
            agreement_id,
            total_charged: row.total_charged,
            total_paid: row.total_paid,
            outstanding: row.total_charged - row.total_paid,
            last_payment_at: row.last_payment_at,
        })
    }
}
