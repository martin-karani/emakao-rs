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
        tax::{MriRegime, TaxBreakdown, TaxPeriod},
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
    owner_id: Option<Uuid>,
    property_id: Uuid,
    agreement_id: Option<Uuid>,
    resident_id: Option<Uuid>,
    invoice_number: String,
    status: String,
    line_items: serde_json::Value,
    subtotal_kes: Decimal,
    mri_kes: Decimal,
    vat_kes: Decimal,
    wht_kes: Decimal,
    total_kes: Decimal,
    net_payable_kes: Option<Decimal>,
    mri_regime: Option<String>,
    tax_period: Option<time::Date>,
    tax_due_date: Option<time::Date>,
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

        let mri_regime = row.mri_regime.map(|s| match s.as_str() {
            "mri" => MriRegime::Mri,
            "normal_income_tax" => MriRegime::NormalIncomeTax,
            "elected_normal" => MriRegime::ElectedNormal,
            _ => MriRegime::Exempt,
        });

        let tax_period = row.tax_period.map(|d| TaxPeriod {
            year: d.year(),
            month: d.month() as u8,
        });

        Ok(Invoice {
            id: row.id,
            agency_id: row.agency_id,
            owner_id: row.owner_id,
            property_id: row.property_id,
            agreement_id: row.agreement_id,
            resident_id: row.resident_id,
            invoice_number: row.invoice_number,
            status,
            line_items,
            subtotal_kes: row.subtotal_kes,
            tax: TaxBreakdown {
                mri_kes: row.mri_kes,
                vat_kes: row.vat_kes,
                wht_kes: row.wht_kes,
                total_tax_kes: row.mri_kes + row.vat_kes,
                net_payable_kes: row.net_payable_kes.unwrap_or(row.total_kes),
                mri_regime,
                tax_period,
                due_date: row.tax_due_date,
            },
            total_kes: row.total_kes,
            due_date: row.due_date,
            notes: row.notes,
            etims_ref: None, // TODO: map from row if columns added to domain
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            voided_at: row.voided_at,
        })
    }
}

const SELECT: &str = r#"
    SELECT
        id, agency_id, owner_id, property_id, agreement_id, resident_id,
        invoice_number, status::text AS status, line_items,
        subtotal_kes, mri_kes, vat_kes, wht_kes, total_kes,
        net_payable_kes, mri_regime::text AS mri_regime,
        tax_period, tax_due_date,
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
        let rows = sqlx::query_as::<_, InvoiceRow>(&format!(
            "{} WHERE agency_id = $1
              AND ($2::uuid IS NULL OR property_id  = $2)
              AND ($3::uuid IS NULL OR agreement_id = $3)
              AND ($4::uuid IS NULL OR resident_id  = $4)
              AND ($5::text IS NULL OR status       = $5::text::invoice_status)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7",
            SELECT
        ))
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
        let row = sqlx::query_as::<_, InvoiceRow>(&format!(
            "{} WHERE id = $1 AND agency_id = $2",
            SELECT
        ))
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

        // For now, keep tax at 0 during creation.
        // A future pass will use TaxBreakdown::compute here.
        let mri = Decimal::ZERO;
        let vat = Decimal::ZERO;
        let wht = Decimal::ZERO;
        let total = subtotal + vat;

        let items_json = serde_json::to_value(&cmd.line_items)
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        let row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            INSERT INTO invoices (
                id, agency_id, owner_id, property_id, agreement_id, resident_id,
                invoice_number, status, line_items,
                subtotal_kes, mri_kes, vat_kes, wht_kes, total_kes,
                due_date, notes, created_by
            )
            VALUES (
                uuidv7(), $1, $2, $3, $4, $5,
                -- auto-generate invoice number: INV-YYYYMM-<seq>
                'INV-' || to_char(now(), 'YYYYMM') || '-' ||
                    LPAD(nextval('invoice_seq')::text, 5, '0'),
                'draft',
                $6::jsonb, $7, $8, $9, $10, $11, $12::date, $13, $14
            )
            RETURNING
                id, agency_id, owner_id, property_id, agreement_id, resident_id,
                invoice_number, status::text AS status, line_items,
                subtotal_kes, mri_kes, vat_kes, wht_kes, total_kes,
                net_payable_kes, mri_regime::text AS mri_regime,
                tax_period, tax_due_date,
                due_date, notes, voided_at,
                created_by, created_at, updated_at
            "#,
        )
        .bind(cmd.agency_id) // $1
        .bind(cmd.owner_id) // $2
        .bind(cmd.property_id) // $3
        .bind(cmd.agreement_id) // $4
        .bind(cmd.resident_id) // $5
        .bind(items_json) // $6
        .bind(subtotal) // $7
        .bind(mri) // $8
        .bind(vat) // $9
        .bind(wht) // $10
        .bind(total) // $11
        .bind(cmd.due_date) // $12
        .bind(cmd.notes) // $13
        .bind(cmd.created_by) // $14
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
                id, agency_id, owner_id, property_id, agreement_id, resident_id,
                invoice_number, status::text AS status, line_items,
                subtotal_kes, mri_kes, vat_kes, wht_kes, total_kes,
                net_payable_kes, mri_regime::text AS mri_regime,
                tax_period, tax_due_date,
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
                id, agency_id, owner_id, property_id, agreement_id, resident_id,
                invoice_number, status::text AS status, line_items,
                subtotal_kes, mri_kes, vat_kes, wht_kes, total_kes,
                net_payable_kes, mri_regime::text AS mri_regime,
                tax_period, tax_due_date,
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
