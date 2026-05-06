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

#[derive(sqlx::FromRow)]
struct LedgerEntryRow {
    id: Uuid,
    agreement_id: Option<Uuid>,
    unit_id: Option<Uuid>,
    resident_id: Option<Uuid>,
    owner_id: Option<Uuid>,
    entry_type: String,
    amount_kes: Decimal,
    description: String,
    external_ref: Option<String>,
    mpesa_receipt: Option<String>,
    period_start: Option<time::Date>,
    period_end: Option<time::Date>,
    posted_by: Uuid,
    posted_at: time::OffsetDateTime,
    is_reconciled: bool,
    metadata: sqlx::types::Json<serde_json::Value>,
}

fn entry_type_to_str(e: &LedgerEntryType) -> &'static str {
    match e {
        LedgerEntryType::Rent => "rent",
        LedgerEntryType::Deposit => "deposit",
        LedgerEntryType::HoaDues => "hoa_dues",
        LedgerEntryType::CamCharge => "cam_charge",
        LedgerEntryType::Utility => "utility",
        LedgerEntryType::MaintenanceCharge => "maintenance_charge",
        LedgerEntryType::LateFee => "late_fee",
        LedgerEntryType::LegalFee => "legal_fee",
        LedgerEntryType::Penalty => "penalty",
        LedgerEntryType::PaymentMpesa => "payment_mpesa",
        LedgerEntryType::PaymentBank => "payment_bank",
        LedgerEntryType::PaymentCash => "payment_cash",
        LedgerEntryType::DepositRefund => "deposit_refund",
        LedgerEntryType::CreditNote => "credit_note",
        LedgerEntryType::Waiver => "waiver",
        LedgerEntryType::Disbursement => "disbursement",
        LedgerEntryType::JournalAdjustment => "journal_adjustment",
    }
}

fn str_to_entry_type(s: &str) -> LedgerEntryType {
    match s {
        "rent" => LedgerEntryType::Rent,
        "deposit" => LedgerEntryType::Deposit,
        "hoa_dues" => LedgerEntryType::HoaDues,
        "cam_charge" => LedgerEntryType::CamCharge,
        "utility" => LedgerEntryType::Utility,
        "maintenance_charge" => LedgerEntryType::MaintenanceCharge,
        "late_fee" => LedgerEntryType::LateFee,
        "legal_fee" => LedgerEntryType::LegalFee,
        "penalty" => LedgerEntryType::Penalty,
        "payment_mpesa" => LedgerEntryType::PaymentMpesa,
        "payment_bank" => LedgerEntryType::PaymentBank,
        "payment_cash" => LedgerEntryType::PaymentCash,
        "deposit_refund" => LedgerEntryType::DepositRefund,
        "credit_note" => LedgerEntryType::CreditNote,
        "waiver" => LedgerEntryType::Waiver,
        "disbursement" => LedgerEntryType::Disbursement,
        _ => LedgerEntryType::JournalAdjustment,
    }
}

impl From<LedgerEntryRow> for LedgerEntry {
    fn from(r: LedgerEntryRow) -> Self {
        Self {
            id: r.id,
            agreement_id: r.agreement_id,
            unit_id: r.unit_id,
            resident_id: r.resident_id,
            owner_id: r.owner_id,
            entry_type: str_to_entry_type(&r.entry_type),
            amount_kes: r.amount_kes,
            description: r.description,
            external_ref: r.external_ref,
            mpesa_receipt: r.mpesa_receipt,
            period_start: r.period_start,
            period_end: r.period_end,
            posted_by: r.posted_by,
            posted_at: r.posted_at,
            is_reconciled: r.is_reconciled,
            metadata: r.metadata.0,
        }
    }
}

#[async_trait]
impl LedgerRepository for PgLedgerRepo {
    async fn create(&self, cmd: CreateLedgerEntryCommand) -> Result<LedgerEntry, AppError> {
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
                entry_type, amount_kes, description, external_ref,
                mpesa_receipt, period_start, period_end, posted_by,
                posted_at, is_reconciled,
                metadata AS "metadata: sqlx::types::Json<serde_json::Value>"
            "#,
            Uuid::new_v4(),
            cmd.agreement_id,
            cmd.unit_id,
            cmd.resident_id,
            cmd.owner_id,
            entry_type_to_str(&cmd.entry_type),
            cmd.amount_kes,
            cmd.description,
            cmd.external_ref,
            cmd.mpesa_receipt,
            cmd.period_start,
            cmd.period_end,
            cmd.posted_by,
            sqlx::types::Json(cmd.metadata) as _
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(LedgerEntry::from(row))
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
                entry_type, amount_kes, description, external_ref,
                mpesa_receipt, period_start, period_end, posted_by,
                posted_at, is_reconciled,
                metadata AS "metadata: sqlx::types::Json<serde_json::Value>"
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

        Ok(rows.into_iter().map(LedgerEntry::from).collect())
    }

    async fn balance_for_agreement(&self, agreement_id: Uuid) -> Result<BalanceSummary, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT
                COALESCE(SUM(amount_kes) FILTER (
                    WHERE entry_type NOT IN (
                        'payment_mpesa','payment_bank','payment_cash',
                        'credit_note','waiver','deposit_refund'
                    )
                ), 0)  AS "total_charged!: rust_decimal::Decimal",

                COALESCE(SUM(amount_kes) FILTER (
                    WHERE entry_type IN (
                        'payment_mpesa','payment_bank','payment_cash',
                        'credit_note','waiver','deposit_refund'
                    )
                ), 0)  AS "total_paid!: rust_decimal::Decimal",

                MAX(posted_at) FILTER (
                    WHERE entry_type IN (
                        'payment_mpesa','payment_bank','payment_cash'
                    )
                ) AS last_payment_at

            FROM ledger_entries
            WHERE agreement_id = $1
            "#,
            agreement_id
        )
        .fetch_one(&self.pool)
        .await?;

        let total_charged = row.total_charged;
        let total_paid = row.total_paid;
        let outstanding = total_charged - total_paid;

        Ok(BalanceSummary {
            agreement_id,
            total_charged,
            total_paid,
            outstanding,
            last_payment_at: row.last_payment_at,
        })
    }
}
