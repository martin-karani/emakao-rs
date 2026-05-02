use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::utility_repository::UtilityRepository},
    domain::utility::{
        BillingMode, CreateMeterCommand, CreateMeterReadingCommand, MeterReading, MeterType,
        UtilityBill, UtilityBillStatus, UtilityMeter,
    },
};

pub struct PgUtilityRepo {
    pool: PgPool,
}

impl PgUtilityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgUtilityRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Meter ─────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MeterRow {
    id: Uuid,
    unit_id: Uuid,
    meter_type: String,
    billing_mode: String,
    meter_number: String,
    rate_per_unit: rust_decimal::Decimal,
    created_at: time::OffsetDateTime,
}

fn parse_meter_type(s: &str) -> MeterType {
    match s {
        "water" => MeterType::Water,
        "gas" => MeterType::Gas,
        _ => MeterType::Electricity,
    }
}

fn meter_type_str(m: &MeterType) -> &'static str {
    match m {
        MeterType::Electricity => "electricity",
        MeterType::Water => "water",
        MeterType::Gas => "gas",
    }
}

fn parse_billing_mode(s: &str) -> BillingMode {
    match s {
        "postpaid" => BillingMode::Postpaid,
        _ => BillingMode::Prepaid,
    }
}

fn billing_mode_str(m: &BillingMode) -> &'static str {
    match m {
        BillingMode::Prepaid => "prepaid",
        BillingMode::Postpaid => "postpaid",
    }
}

impl From<MeterRow> for UtilityMeter {
    fn from(r: MeterRow) -> Self {
        Self {
            id: r.id,
            unit_id: r.unit_id,
            meter_type: parse_meter_type(&r.meter_type),
            billing_mode: parse_billing_mode(&r.billing_mode),
            meter_number: r.meter_number,
            rate_per_unit: r.rate_per_unit,
            created_at: r.created_at,
        }
    }
}

// ── Reading ───────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct ReadingRow {
    id: Uuid,
    meter_id: Uuid,
    reading_value: rust_decimal::Decimal,
    read_at: time::OffsetDateTime,
    recorded_by: Uuid,
}

impl From<ReadingRow> for MeterReading {
    fn from(r: ReadingRow) -> Self {
        Self {
            id: r.id,
            meter_id: r.meter_id,
            reading_value: r.reading_value,
            read_at: r.read_at,
            recorded_by: r.recorded_by,
        }
    }
}

// ── Bill ──────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct BillRow {
    id: Uuid,
    meter_id: Uuid,
    unit_id: Uuid,
    units_consumed: rust_decimal::Decimal,
    amount_kes: rust_decimal::Decimal,
    status: String,
    created_at: time::OffsetDateTime,
}

fn parse_bill_status(s: &str) -> UtilityBillStatus {
    match s {
        "issued" => UtilityBillStatus::Issued,
        "paid" => UtilityBillStatus::Paid,
        "overdue" => UtilityBillStatus::Overdue,
        _ => UtilityBillStatus::Draft,
    }
}

impl From<BillRow> for UtilityBill {
    fn from(r: BillRow) -> Self {
        Self {
            id: r.id,
            meter_id: r.meter_id,
            unit_id: r.unit_id,
            units_consumed: r.units_consumed,
            amount_kes: r.amount_kes,
            status: parse_bill_status(&r.status),
            created_at: r.created_at,
        }
    }
}

// ── Repository impl ───────────────────────────────────────────────────────────

#[async_trait]
impl UtilityRepository for PgUtilityRepo {
    async fn find_meter_by_id(&self, id: Uuid) -> Result<Option<UtilityMeter>, AppError> {
        let row = sqlx::query_as!(
            MeterRow,
            r#"
            SELECT id, unit_id, meter_type, billing_mode,
                   meter_number, rate_per_unit, created_at
            FROM utility_meters
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(UtilityMeter::from))
    }

    async fn create_meter(&self, cmd: CreateMeterCommand) -> Result<UtilityMeter, AppError> {
        let row = sqlx::query_as!(
            MeterRow,
            r#"
            INSERT INTO utility_meters (
                id, unit_id, meter_type, billing_mode, meter_number, rate_per_unit
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, unit_id, meter_type, billing_mode,
                      meter_number, rate_per_unit, created_at
            "#,
            Uuid::new_v4(),
            cmd.unit_id,
            meter_type_str(&cmd.meter_type),
            billing_mode_str(&cmd.billing_mode),
            cmd.meter_number,
            cmd.rate_per_unit
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(UtilityMeter::from(row))
    }

    async fn create_reading(
        &self,
        cmd: CreateMeterReadingCommand,
    ) -> Result<MeterReading, AppError> {
        let row = sqlx::query_as!(
            ReadingRow,
            r#"
            INSERT INTO meter_readings (id, meter_id, reading_value, recorded_by)
            VALUES ($1, $2, $3, $4)
            RETURNING id, meter_id, reading_value, read_at, recorded_by
            "#,
            Uuid::new_v4(),
            cmd.meter_id,
            cmd.reading_value,
            cmd.recorded_by
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(MeterReading::from(row))
    }

    async fn last_two_readings(&self, meter_id: Uuid) -> Result<Vec<MeterReading>, AppError> {
        let rows = sqlx::query_as!(
            ReadingRow,
            r#"
            SELECT id, meter_id, reading_value, read_at, recorded_by
            FROM meter_readings
            WHERE meter_id = $1
            ORDER BY read_at DESC
            LIMIT 2
            "#,
            meter_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(MeterReading::from).collect())
    }

    async fn create_bill(&self, bill: UtilityBill) -> Result<UtilityBill, AppError> {
        let row = sqlx::query_as!(
            BillRow,
            r#"
            INSERT INTO utility_bills (
                id, meter_id, unit_id, units_consumed, amount_kes, status
            )
            VALUES ($1, $2, $3, $4, $5, 'draft')
            RETURNING id, meter_id, unit_id, units_consumed, amount_kes, status, created_at
            "#,
            bill.id,
            bill.meter_id,
            bill.unit_id,
            bill.units_consumed,
            bill.amount_kes
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(UtilityBill::from(row))
    }
}
