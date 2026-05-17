use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::invoice_repository::{CreateInvoiceCommand, InvoiceFilter, InvoiceRepository},
    },
    domain::{
        enums::InvoiceStatus,
        invoice::{Invoice, InvoiceLineItem},
    },
};

pub struct PgInvoiceRepo {
    pool: PgPool,
}

impl PgInvoiceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct InvoiceRow {
    id: Uuid,
    agency_id: Uuid,
    property_id: Uuid,
    agreement_id: Option<Uuid>,
    resident_id: Option<Uuid>,
    invoice_number: String,
    status: String,
    line_items: serde_json::Value,
    subtotal_kes: Decimal,
    tax_kes: Decimal,
    total_kes: Decimal,
    due_date: time::Date,
    notes: Option<String>,
    voided_at: Option<time::OffsetDateTime>,
    created_by: Uuid,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

impl TryFrom<InvoiceRow> for Invoice {
    type Error = AppError;

    fn try_from(row: InvoiceRow) -> Result<Self, Self::Error> {
        let status: InvoiceStatus = serde_json::from_value(serde_json::Value::String(row.status))
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        let line_items: Vec<InvoiceLineItem> = serde_json::from_value(row.line_items)
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        Ok(Invoice {
            id: row.id,
            agency_id: row.agency_id,
            property_id: row.property_id,
            agreement_id: row.agreement_id,
            resident_id: row.resident_id,
            invoice_number: row.invoice_number,
            status,
            line_items,
            subtotal_kes: row.subtotal_kes,
            tax_kes: row.tax_kes,
            total_kes: row.total_kes,
            due_date: row.due_date,
            notes: row.notes,
            voided_at: row.voided_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

const SELECT: &str = r#"
    SELECT
        id, agency_id, property_id, agreement_id, resident_id,
        invoice_number, status, line_items,
        subtotal_kes, tax_kes, total_kes,
        due_date, notes, voided_at,
        created_by, created_at, updated_at
    FROM invoices
"#;

#[async_trait]
impl InvoiceRepository for PgInvoiceRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        filter: InvoiceFilter,
    ) -> Result<Vec<Invoice>, AppError> {
        let rows = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT
                id, agency_id, property_id, agreement_id, resident_id,
                invoice_number, status::text AS status, line_items,
                subtotal_kes, tax_kes, total_kes,
                due_date, notes, voided_at,
                created_by, created_at, updated_at
            FROM invoices
            WHERE agency_id = $1
              AND ($2::uuid IS NULL OR property_id  = $2)
              AND ($3::uuid IS NULL OR agreement_id = $3)
              AND ($4::uuid IS NULL OR resident_id  = $4)
              AND ($5::text IS NULL OR status       = $5::text::invoice_status)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(agency_id)
        .bind(filter.property_id)
        .bind(filter.agreement_id)
        .bind(filter.resident_id)
        .bind(filter.status.as_ref().map(invoice_status_str))
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        rows.into_iter().map(Invoice::try_from).collect()
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Invoice>, AppError> {
        let row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT
                id, agency_id, property_id, agreement_id, resident_id,
                invoice_number, status::text AS status, line_items,
                subtotal_kes, tax_kes, total_kes,
                due_date, notes, voided_at,
                created_by, created_at, updated_at
            FROM invoices
            WHERE id = $1 AND agency_id = $2
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        row.map(Invoice::try_from).transpose()
    }

    async fn create(&self, cmd: CreateInvoiceCommand) -> Result<Invoice, AppError> {
        // Compute totals from line items (VAT at 0% for now — configurable later)
        let subtotal: Decimal = cmd
            .line_items
            .iter()
            .map(|li| li.quantity * li.unit_price_kes)
            .sum();
        let tax = Decimal::ZERO;
        let total = subtotal + tax;

        let items_json = serde_json::to_value(&cmd.line_items)
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        let row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            INSERT INTO invoices (
                id, agency_id, property_id, agreement_id, resident_id,
                invoice_number, status, line_items,
                subtotal_kes, tax_kes, total_kes,
                due_date, notes, created_by
            )
            VALUES (
                uuidv7(), $1, $2, $3, $4,
                -- auto-generate invoice number: INV-YYYYMM-<seq>
                'INV-' || to_char(now(), 'YYYYMM') || '-' ||
                    LPAD(nextval('invoice_seq')::text, 5, '0'),
                'draft',
                $5::jsonb, $6, $7, $8, $9::date, $10, $11
            )
            RETURNING
                id, agency_id, property_id, agreement_id, resident_id,
                invoice_number, status::text AS status, line_items,
                subtotal_kes, tax_kes, total_kes,
                due_date, notes, voided_at,
                created_by, created_at, updated_at
            "#,
        )
        .bind(cmd.agency_id)    // $1
        .bind(cmd.property_id)  // $2
        .bind(cmd.agreement_id) // $3
        .bind(cmd.resident_id)  // $4
        .bind(items_json)       // $5
        .bind(subtotal)         // $6
        .bind(tax)              // $7
        .bind(total)            // $8
        .bind(cmd.due_date)     // $9
        .bind(cmd.notes)        // $10
        .bind(cmd.created_by)   // $11
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Invoice::try_from(row)
    }

    async fn update_status(&self, id: Uuid, status: InvoiceStatus) -> Result<Invoice, AppError> {
        let row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            UPDATE invoices
            SET status = $2::text::invoice_status, updated_at = now()
            WHERE id = $1
            RETURNING
                id, agency_id, property_id, agreement_id, resident_id,
                invoice_number, status::text AS status, line_items,
                subtotal_kes, tax_kes, total_kes,
                due_date, notes, voided_at,
                created_by, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(invoice_status_str(&status))
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("invoice {id}")))?;

        Invoice::try_from(row)
    }

    async fn void(&self, id: Uuid) -> Result<Invoice, AppError> {
        let row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            UPDATE invoices
            SET status     = 'void',
                voided_at  = now(),
                updated_at = now()
            WHERE id = $1
            RETURNING
                id, agency_id, property_id, agreement_id, resident_id,
                invoice_number, status::text AS status, line_items,
                subtotal_kes, tax_kes, total_kes,
                due_date, notes, voided_at,
                created_by, created_at, updated_at
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("invoice {id}")))?;

        Invoice::try_from(row)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn invoice_status_str(s: &InvoiceStatus) -> &'static str {
    match s {
        InvoiceStatus::Draft => "draft",
        InvoiceStatus::Sent => "sent",
        InvoiceStatus::Paid => "paid",
        InvoiceStatus::Overdue => "overdue",
        InvoiceStatus::Void => "void",
    }
}
