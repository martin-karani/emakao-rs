use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::utility_repository::UtilityRepository},
    domain::{
        enums::{BillingMode, MeterType, UtilityBillStatus},
        utility::{
            CreateMeterCommand, CreateMeterReadingCommand, MeterReading, UtilityBill, UtilityMeter,
        },
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

fn bill_status_str(s: &UtilityBillStatus) -> &'static str {
    match s {
        UtilityBillStatus::Draft => "draft",
        UtilityBillStatus::Issued => "issued",
        UtilityBillStatus::Paid => "paid",
        UtilityBillStatus::Overdue => "overdue",
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
        let row = sqlx::query_as::<_, MeterRow>(
            r#"
            SELECT id, unit_id, meter_type::text, billing_mode::text,
                   meter_number, rate_per_unit, created_at
            FROM utility_meters
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(UtilityMeter::from))
    }

    async fn find_meters_by_unit(&self, unit_id: Uuid) -> Result<Vec<UtilityMeter>, AppError> {
        let rows = sqlx::query_as::<_, MeterRow>(
            r#"
            SELECT id, unit_id, meter_type::text, billing_mode::text,
                   meter_number, rate_per_unit, created_at
            FROM utility_meters
            WHERE unit_id = $1
            "#,
        )
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(UtilityMeter::from).collect())
    }

    async fn find_bills_by_unit(
        &self,
        unit_id: Uuid,
        status: Option<UtilityBillStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UtilityBill>, AppError> {
        let status_str = status.as_ref().map(bill_status_str);

        let rows = sqlx::query_as::<_, BillRow>(
            r#"
            SELECT id, meter_id, unit_id, units_consumed, amount_kes, status::text, created_at
            FROM utility_bills
            WHERE unit_id = $1
              AND ($2::text IS NULL OR status::text = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(unit_id)
        .bind(status_str)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(UtilityBill::from).collect())
    }

    async fn create_meter(&self, cmd: CreateMeterCommand) -> Result<UtilityMeter, AppError> {
        let row = sqlx::query_as::<_, MeterRow>(
            r#"
            INSERT INTO utility_meters (
                id, unit_id, meter_type, billing_mode, meter_number, rate_per_unit
            )
            VALUES ($1, $2, $3::meter_type, $4::billing_mode, $5, $6)
            RETURNING id, unit_id, meter_type::text, billing_mode::text,
                      meter_number, rate_per_unit, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(cmd.unit_id)
        .bind(meter_type_str(&cmd.meter_type))
        .bind(billing_mode_str(&cmd.billing_mode))
        .bind(cmd.meter_number)
        .bind(cmd.rate_per_unit)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(UtilityMeter::from(row))
    }

    async fn create_reading(
        &self,
        cmd: CreateMeterReadingCommand,
    ) -> Result<MeterReading, AppError> {
        let row = sqlx::query_as::<_, ReadingRow>(
            r#"
            INSERT INTO meter_readings (id, meter_id, reading_value, recorded_by)
            VALUES ($1, $2, $3, $4)
            RETURNING id, meter_id, reading_value, read_at, recorded_by
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(cmd.meter_id)
        .bind(cmd.reading_value)
        .bind(cmd.recorded_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(MeterReading::from(row))
    }

    async fn last_two_readings(&self, meter_id: Uuid) -> Result<Vec<MeterReading>, AppError> {
        let rows = sqlx::query_as::<_, ReadingRow>(
            r#"
            SELECT id, meter_id, reading_value, read_at, recorded_by
            FROM meter_readings
            WHERE meter_id = $1
            ORDER BY read_at DESC
            LIMIT 2
            "#,
        )
        .bind(meter_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(MeterReading::from).collect())
    }

    async fn create_bill(&self, bill: UtilityBill) -> Result<UtilityBill, AppError> {
        let row = sqlx::query_as::<_, BillRow>(
            r#"
            INSERT INTO utility_bills (
                id, meter_id, unit_id, units_consumed, amount_kes, status
            )
            VALUES ($1, $2, $3, $4, $5, 'draft'::utility_bill_status)
            RETURNING id, meter_id, unit_id, units_consumed, amount_kes, status::text, created_at
            "#,
        )
        .bind(bill.id)
        .bind(bill.meter_id)
        .bind(bill.unit_id)
        .bind(bill.units_consumed)
        .bind(bill.amount_kes)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(UtilityBill::from(row))
    }
}
