use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::tax_repository::{TaxObligationFilter, TaxRepository},
    },
    domain::tax::{
        CreateTaxObligationCommand, MarkTaxFiledCommand, MarkTaxPaidCommand, MriRegime,
        OwnerAnnualRentalIncome, TaxComplianceSummary, TaxObligation, TaxObligationStatus,
        TaxObligationType, TaxPeriod,
    },
};

// ── Helper: string → enum conversions ────────────────────────────────────────

fn parse_obligation_type(s: &str) -> TaxObligationType {
    match s {
        "vat" => TaxObligationType::Vat,
        "wht" => TaxObligationType::Wht,
        _ => TaxObligationType::Mri,
    }
}

fn parse_obligation_status(s: &str) -> TaxObligationStatus {
    match s {
        "filed" => TaxObligationStatus::Filed,
        "paid" => TaxObligationStatus::Paid,
        "overdue" => TaxObligationStatus::Overdue,
        "nil_filed" => TaxObligationStatus::NilFiled,
        _ => TaxObligationStatus::Pending,
    }
}

fn parse_mri_regime(s: &str) -> MriRegime {
    match s {
        "mri" => MriRegime::Mri,
        "normal_income_tax" => MriRegime::NormalIncomeTax,
        "elected_normal" => MriRegime::ElectedNormal,
        _ => MriRegime::Exempt,
    }
}

// ── Row → domain mapping ──────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct TaxObligationRow {
    id: Uuid,
    agency_id: Uuid,
    owner_id: Uuid,
    property_id: Uuid,
    agreement_id: Option<Uuid>,
    obligation_type: String,
    tax_period: Date,
    gross_amount_kes: Decimal,
    tax_kes: Decimal,
    tax_rate: Decimal,
    due_date: Date,
    status: String,
    kra_prn: Option<String>,
    kra_ack_number: Option<String>,
    filed_at: Option<OffsetDateTime>,
    remitted_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct AnnualIncomeRow {
    owner_id: Uuid,
    year: i16,
    total_gross_rent_kes: Decimal,
    mri_regime: String,
    elected_normal_regime: bool,
}

impl TaxObligationRow {
    fn into_domain(self) -> TaxObligation {
        let tp = TaxPeriod {
            year: self.tax_period.year(),
            month: self.tax_period.month() as u8,
        };
        TaxObligation {
            id: self.id,
            agency_id: self.agency_id,
            owner_id: self.owner_id,
            property_id: self.property_id,
            agreement_id: self.agreement_id,
            obligation_type: parse_obligation_type(&self.obligation_type),
            tax_period: tp,
            gross_amount_kes: self.gross_amount_kes,
            tax_kes: self.tax_kes,
            tax_rate: self.tax_rate,
            due_date: self.due_date,
            status: parse_obligation_status(&self.status),
            kra_prn: self.kra_prn,
            kra_ack_number: self.kra_ack_number,
            filed_at: self.filed_at,
            remitted_at: self.remitted_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

// ── Repository ────────────────────────────────────────────────────────────────

pub struct PgTaxRepo {
    pool: PgPool,
    _agency_id: Uuid,
}

impl PgTaxRepo {
    pub fn new(pool: PgPool, agency_id: Uuid) -> Self {
        Self {
            pool,
            _agency_id: agency_id,
        }
    }
}

#[async_trait]
impl TaxRepository for PgTaxRepo {
    async fn create_obligation(
        &self,
        cmd: CreateTaxObligationCommand,
    ) -> Result<TaxObligation, AppError> {
        let period_date = Date::from_calendar_date(
            cmd.tax_period.year,
            time::Month::try_from(cmd.tax_period.month)
                .map_err(|e| AppError::InternalServer(format!("invalid month: {e}")))?,
            1,
        )
        .map_err(|e| AppError::InternalServer(format!("invalid period date: {e}")))?;

        let row = sqlx::query_as::<_, TaxObligationRow>(
            r#"
            INSERT INTO tax_obligations (
                agency_id, owner_id, property_id, agreement_id,
                obligation_type, tax_period, gross_amount_kes,
                tax_kes, tax_rate, due_date, status
            )
            VALUES ($1, $2, $3, $4, $5::tax_obligation_type, $6, $7, $8, $9, $10, 'pending'::tax_obligation_status)
            ON CONFLICT (agency_id, owner_id, property_id, tax_period, obligation_type)
                DO UPDATE SET updated_at = NOW()
            RETURNING
                id, agency_id, owner_id, property_id, agreement_id,
                obligation_type::text, tax_period, gross_amount_kes,
                tax_kes, tax_rate, due_date, status::text,
                kra_prn, kra_ack_number, filed_at, remitted_at,
                created_at, updated_at
            "#,
        )
        .bind(cmd.agency_id)
        .bind(cmd.owner_id)
        .bind(cmd.property_id)
        .bind(cmd.agreement_id)
        .bind(cmd.obligation_type.to_string().to_lowercase())
        .bind(period_date)
        .bind(cmd.gross_amount_kes)
        .bind(cmd.tax_kes)
        .bind(cmd.tax_rate)
        .bind(cmd.due_date)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("create_obligation: {e}")))?;

        Ok(row.into_domain())
    }

    async fn find_obligation_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<TaxObligation>, AppError> {
        let row = sqlx::query_as::<_, TaxObligationRow>(
            r#"
            SELECT
                id, agency_id, owner_id, property_id, agreement_id,
                obligation_type::text, tax_period, gross_amount_kes,
                tax_kes, tax_rate, due_date, status::text,
                kra_prn, kra_ack_number, filed_at, remitted_at,
                created_at, updated_at
            FROM tax_obligations
            WHERE id = $1 AND agency_id = $2
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("find_obligation_by_id: {e}")))?;

        Ok(row.map(|r| r.into_domain()))
    }

    async fn list_obligations(
        &self,
        filter: TaxObligationFilter,
    ) -> Result<Vec<TaxObligation>, AppError> {
        // Build a dynamic query; sqlx doesn't support fully dynamic WHERE so we
        // use a raw query string with explicit casts.
        let rows = sqlx::query_as::<_, TaxObligationRow>(
            r#"
            SELECT
                id, agency_id, owner_id, property_id, agreement_id,
                obligation_type::text, tax_period, gross_amount_kes,
                tax_kes, tax_rate, due_date, status::text,
                kra_prn, kra_ack_number, filed_at, remitted_at,
                created_at, updated_at
            FROM tax_obligations
            WHERE agency_id = $1
              AND ($2::uuid     IS NULL OR owner_id        = $2)
              AND ($3::uuid     IS NULL OR property_id     = $3)
              AND ($4::text     IS NULL OR obligation_type = $4::tax_obligation_type)
              AND ($5::text     IS NULL OR status          = $5::tax_obligation_status)
              AND ($6::date     IS NULL OR tax_period      = $6)
              AND ($7::date     IS NULL OR due_date       <= $7)
            ORDER BY tax_period DESC, due_date ASC
            LIMIT  $8
            OFFSET $9
            "#,
        )
        .bind(filter.agency_id)
        .bind(filter.owner_id)
        .bind(filter.property_id)
        .bind(filter.obligation_type.map(|t| t.to_string().to_lowercase()))
        .bind(filter.status.map(|s| format!("{s:?}").to_lowercase()))
        .bind(filter.tax_period)
        .bind(filter.due_on_or_before)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("list_obligations: {e}")))?;

        Ok(rows.into_iter().map(|r| r.into_domain()).collect())
    }

    async fn mark_filed(&self, cmd: MarkTaxFiledCommand) -> Result<TaxObligation, AppError> {
        let row = sqlx::query_as::<_, TaxObligationRow>(
            r#"
            UPDATE tax_obligations
            SET
                status         = 'filed'::tax_obligation_status,
                kra_ack_number = $2,
                filed_at       = $3,
                updated_at     = NOW()
            WHERE id = $1
            RETURNING
                id, agency_id, owner_id, property_id, agreement_id,
                obligation_type::text, tax_period, gross_amount_kes,
                tax_kes, tax_rate, due_date, status::text,
                kra_prn, kra_ack_number, filed_at, remitted_at,
                created_at, updated_at
            "#,
        )
        .bind(cmd.obligation_id)
        .bind(cmd.kra_ack_number)
        .bind(cmd.filed_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("mark_filed: {e}")))?;

        Ok(row.into_domain())
    }

    async fn mark_paid(&self, cmd: MarkTaxPaidCommand) -> Result<TaxObligation, AppError> {
        let row = sqlx::query_as::<_, TaxObligationRow>(
            r#"
            UPDATE tax_obligations
            SET
                status      = 'paid'::tax_obligation_status,
                kra_prn     = $2,
                remitted_at = $3,
                updated_at  = NOW()
            WHERE id = $1
            RETURNING
                id, agency_id, owner_id, property_id, agreement_id,
                obligation_type::text, tax_period, gross_amount_kes,
                tax_kes, tax_rate, due_date, status::text,
                kra_prn, kra_ack_number, filed_at, remitted_at,
                created_at, updated_at
            "#,
        )
        .bind(cmd.obligation_id)
        .bind(cmd.kra_prn)
        .bind(cmd.remitted_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("mark_paid: {e}")))?;

        Ok(row.into_domain())
    }

    async fn mark_overdue_batch(&self, agency_id: Uuid, as_of: Date) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE tax_obligations
            SET status = 'overdue'::tax_obligation_status, updated_at = NOW()
            WHERE agency_id = $1
              AND due_date  < $2
              AND status IN ('pending'::tax_obligation_status, 'filed'::tax_obligation_status)
            "#,
        )
        .bind(agency_id)
        .bind(as_of)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("mark_overdue_batch: {e}")))?;

        Ok(result.rows_affected())
    }

    async fn upsert_annual_income(
        &self,
        agency_id: Uuid,
        owner_id: Uuid,
        year: i32,
        total_gross_rent_kes: Decimal,
        regime: MriRegime,
    ) -> Result<OwnerAnnualRentalIncome, AppError> {
        let regime_str = format!("{regime:?}")
            .to_lowercase()
            .replace("normalincometax", "normal_income_tax")
            .replace("electedNormal", "elected_normal");

        let row = sqlx::query_as::<_, AnnualIncomeRow>(
            r#"
            INSERT INTO owner_annual_rental_income
                (agency_id, owner_id, year, total_gross_rent_kes, mri_regime)
            VALUES ($1, $2, $3, $4, $5::mri_regime)
            ON CONFLICT (agency_id, owner_id, year)
                DO UPDATE SET
                    total_gross_rent_kes = $4,
                    mri_regime           = $5::mri_regime,
                    updated_at           = NOW()
            RETURNING
                owner_id, year,
                total_gross_rent_kes, mri_regime::text,
                elected_normal_regime
            "#,
        )
        .bind(agency_id)
        .bind(owner_id)
        .bind(year as i16)
        .bind(total_gross_rent_kes)
        .bind(regime_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("upsert_annual_income: {e}")))?;

        Ok(OwnerAnnualRentalIncome {
            owner_id,
            year,
            total_gross_rent_kes: row.total_gross_rent_kes,
            mri_regime: parse_mri_regime(&row.mri_regime),
            elected_normal_regime: row.elected_normal_regime,
        })
    }

    async fn get_annual_income(
        &self,
        agency_id: Uuid,
        owner_id: Uuid,
        year: i32,
    ) -> Result<Option<OwnerAnnualRentalIncome>, AppError> {
        let row = sqlx::query_as::<_, AnnualIncomeRow>(
            r#"
            SELECT
                owner_id, year,
                total_gross_rent_kes, mri_regime::text AS mri_regime,
                elected_normal_regime
            FROM owner_annual_rental_income
            WHERE agency_id = $1 AND owner_id = $2 AND year = $3
            "#,
        )
        .bind(agency_id)
        .bind(owner_id)
        .bind(year as i16)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("get_annual_income: {e}")))?;

        Ok(row.map(|r| OwnerAnnualRentalIncome {
            owner_id: r.owner_id,
            year: r.year as i32,
            total_gross_rent_kes: r.total_gross_rent_kes,
            mri_regime: parse_mri_regime(&r.mri_regime),
            elected_normal_regime: r.elected_normal_regime,
        }))
    }

    async fn get_compliance_summary(
        &self,
        agency_id: Uuid,
        tax_period: TaxPeriod,
    ) -> Result<TaxComplianceSummary, AppError> {
        let period_date = Date::from_calendar_date(
            tax_period.year,
            time::Month::try_from(tax_period.month)
                .map_err(|e| AppError::InternalServer(format!("invalid month: {e}")))?,
            1,
        )
        .map_err(|e| AppError::InternalServer(format!("invalid period date: {e}")))?;

        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN obligation_type = 'mri' THEN tax_kes ELSE 0 END), 0) AS total_mri_kes,
                COALESCE(SUM(CASE WHEN obligation_type = 'vat' THEN tax_kes ELSE 0 END), 0) AS total_vat_kes,
                COALESCE(SUM(CASE WHEN obligation_type = 'wht' THEN tax_kes ELSE 0 END), 0) AS total_wht_kes,
                COUNT(*)                                                                    AS obligations_total,
                COUNT(*) FILTER (WHERE status IN ('filed','paid'))                          AS obligations_filed,
                COUNT(*) FILTER (WHERE status = 'paid')                                     AS obligations_paid,
                COUNT(*) FILTER (WHERE status = 'overdue')                                  AS obligations_overdue
            FROM tax_obligations
            WHERE agency_id = $1
              AND tax_period = $2
            "#,
        )
        .bind(agency_id)
        .bind(period_date)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("get_compliance_summary: {e}")))?;

        use sqlx::Row;
        Ok(TaxComplianceSummary {
            agency_id,
            tax_period,
            total_mri_kes: row
                .get::<Option<Decimal>, _>("total_mri_kes")
                .unwrap_or(Decimal::ZERO),
            total_vat_kes: row
                .get::<Option<Decimal>, _>("total_vat_kes")
                .unwrap_or(Decimal::ZERO),
            total_wht_kes: row
                .get::<Option<Decimal>, _>("total_wht_kes")
                .unwrap_or(Decimal::ZERO),
            obligations_total: row.get::<Option<i64>, _>("obligations_total").unwrap_or(0) as u64,
            obligations_filed: row.get::<Option<i64>, _>("obligations_filed").unwrap_or(0) as u64,
            obligations_paid: row.get::<Option<i64>, _>("obligations_paid").unwrap_or(0) as u64,
            obligations_overdue: row
                .get::<Option<i64>, _>("obligations_overdue")
                .unwrap_or(0) as u64,
            mri_due_date: tax_period.mri_due_date(),
            vat_due_date: tax_period.mri_due_date(), // same deadline
        })
    }

    async fn obligation_exists(
        &self,
        agency_id: Uuid,
        owner_id: Uuid,
        property_id: Uuid,
        tax_period: Date,
        obligation_type: TaxObligationType,
    ) -> Result<bool, AppError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM tax_obligations
            WHERE agency_id = $1
              AND owner_id  = $2
              AND property_id = $3
              AND tax_period = $4
              AND obligation_type = $5::tax_obligation_type
            "#,
        )
        .bind(agency_id)
        .bind(owner_id)
        .bind(property_id)
        .bind(tax_period)
        .bind(obligation_type.to_string().to_lowercase())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("obligation_exists: {e}")))?;

        Ok(count.0 > 0)
    }
}
