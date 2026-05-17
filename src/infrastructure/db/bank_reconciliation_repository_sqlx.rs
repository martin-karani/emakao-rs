use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::bank_reconciliation_repository::BankReconciliationRepository,
    },
    domain::bank_reconciliation::{
        BankStatement, BankStatementLine, ImportStatementCommand, MatchLineCommand,
        ReconciliationReport, ReconciliationStatus, UnmatchLineCommand,
    },
};

pub struct PgBankReconciliationRepo {
    pool: PgPool,
}

impl PgBankReconciliationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Row types ─────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct StatementRow {
    id: Uuid,
    agency_id: Uuid,
    bank_name: String,
    account_number: String,
    statement_date: time::Date,
    opening_balance: Decimal,
    closing_balance: Decimal,
    created_by: Uuid,
    created_at: time::OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    statement_id: Uuid,
    value_date: time::Date,
    description: String,
    amount: Decimal,
    reference: Option<String>,
    matched_entry_id: Option<Uuid>,
    created_at: time::OffsetDateTime,
}

fn line_from_row(r: LineRow) -> BankStatementLine {
    let status = if r.matched_entry_id.is_some() {
        ReconciliationStatus::Matched
    } else {
        ReconciliationStatus::Unmatched
    };
    BankStatementLine {
        id: r.id,
        statement_id: r.statement_id,
        value_date: r.value_date,
        description: r.description,
        amount: r.amount,
        reference: r.reference,
        matched_entry_id: r.matched_entry_id,
        status,
        created_at: r.created_at,
    }
}

async fn load_lines(pool: &PgPool, statement_id: Uuid) -> Result<Vec<BankStatementLine>, AppError> {
    let rows = sqlx::query_as::<_, LineRow>(
        r#"SELECT id, statement_id, value_date, description, amount,
                  reference, matched_entry_id, created_at
           FROM   bank_statement_lines
           WHERE  statement_id = $1
           ORDER BY value_date, created_at"#,
    )
    .bind(statement_id)
    .fetch_all(pool)
    .await
    .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
    Ok(rows.into_iter().map(line_from_row).collect())
}

fn statement_from_row(r: StatementRow, lines: Vec<BankStatementLine>) -> BankStatement {
    BankStatement {
        id: r.id,
        agency_id: r.agency_id,
        bank_name: r.bank_name,
        account_number: r.account_number,
        statement_date: r.statement_date,
        opening_balance: r.opening_balance,
        closing_balance: r.closing_balance,
        lines,
        created_by: r.created_by,
        created_at: r.created_at,
    }
}

// ── Repository impl ───────────────────────────────────────────────────────────

#[async_trait]
impl BankReconciliationRepository for PgBankReconciliationRepo {
    async fn list_statements(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BankStatement>, AppError> {
        let rows = sqlx::query_as::<_, StatementRow>(
            r#"SELECT id, agency_id, bank_name, account_number, statement_date,
                      opening_balance, closing_balance, created_by, created_at
               FROM   bank_statements
               WHERE  agency_id = $1
               ORDER  BY statement_date DESC
               LIMIT  $2 OFFSET $3"#,
        )
        .bind(agency_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        // List view returns statements without lines (use get for detail).
        Ok(rows
            .into_iter()
            .map(|r| statement_from_row(r, vec![]))
            .collect())
    }

    async fn find_statement_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<BankStatement>, AppError> {
        let row = sqlx::query_as::<_, StatementRow>(
            r#"SELECT id, agency_id, bank_name, account_number, statement_date,
                      opening_balance, closing_balance, created_by, created_at
               FROM   bank_statements
               WHERE  id = $1 AND agency_id = $2"#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        match row {
            None => Ok(None),
            Some(r) => {
                let lines = load_lines(&self.pool, r.id).await?;
                Ok(Some(statement_from_row(r, lines)))
            }
        }
    }

    async fn import_statement(
        &self,
        cmd: ImportStatementCommand,
    ) -> Result<BankStatement, AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let stmt_row = sqlx::query_as::<_, StatementRow>(
            r#"INSERT INTO bank_statements
                   (id, agency_id, bank_name, account_number, statement_date,
                    opening_balance, closing_balance, created_by)
               VALUES (uuidv7(), $1, $2, $3, $4, $5, $6, $7)
               RETURNING id, agency_id, bank_name, account_number, statement_date,
                         opening_balance, closing_balance, created_by, created_at"#,
        )
        .bind(cmd.agency_id)
        .bind(cmd.bank_name)
        .bind(cmd.account_number)
        .bind(cmd.statement_date)
        .bind(cmd.opening_balance)
        .bind(cmd.closing_balance)
        .bind(cmd.created_by)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let statement_id = stmt_row.id;
        let mut line_rows: Vec<BankStatementLine> = Vec::with_capacity(cmd.lines.len());

        for line in cmd.lines {
            let row = sqlx::query_as::<_, LineRow>(
                r#"INSERT INTO bank_statement_lines
                       (id, statement_id, value_date, description, amount, reference)
                   VALUES (uuidv7(), $1, $2, $3, $4, $5)
                   RETURNING id, statement_id, value_date, description, amount,
                             reference, matched_entry_id, created_at"#,
            )
            .bind(statement_id)
            .bind(line.value_date)
            .bind(line.description)
            .bind(line.amount)
            .bind(line.reference)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

            line_rows.push(line_from_row(row));
        }

        tx.commit()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        tracing::info!(
            statement_id = %statement_id,
            line_count   = line_rows.len(),
            "bank statement imported"
        );

        Ok(statement_from_row(stmt_row, line_rows))
    }

    async fn find_line_by_id(
        &self,
        agency_id: Uuid,
        line_id: Uuid,
    ) -> Result<Option<BankStatementLine>, AppError> {
        let row = sqlx::query_as::<_, LineRow>(
            r#"SELECT bsl.id, bsl.statement_id, bsl.value_date, bsl.description,
                      bsl.amount, bsl.reference, bsl.matched_entry_id, bsl.created_at
               FROM   bank_statement_lines bsl
               JOIN   bank_statements bs ON bs.id = bsl.statement_id
               WHERE  bsl.id = $1 AND bs.agency_id = $2"#,
        )
        .bind(line_id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(row.map(line_from_row))
    }

    async fn match_line(&self, cmd: MatchLineCommand) -> Result<BankStatementLine, AppError> {
        let row = sqlx::query_as::<_, LineRow>(
            r#"UPDATE bank_statement_lines bsl
               SET    matched_entry_id = $1
               FROM   bank_statements bs
               WHERE  bsl.id           = $2
                 AND  bsl.statement_id = bs.id
                 AND  bs.agency_id     = $3
               RETURNING bsl.id, bsl.statement_id, bsl.value_date, bsl.description,
                         bsl.amount, bsl.reference, bsl.matched_entry_id, bsl.created_at"#,
        )
        .bind(cmd.journal_entry_id)
        .bind(cmd.line_id)
        .bind(cmd.agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("bank statement line {}", cmd.line_id)))?;

        Ok(line_from_row(row))
    }

    async fn unmatch_line(&self, cmd: UnmatchLineCommand) -> Result<BankStatementLine, AppError> {
        let row = sqlx::query_as::<_, LineRow>(
            r#"UPDATE bank_statement_lines bsl
               SET    matched_entry_id = NULL
               FROM   bank_statements bs
               WHERE  bsl.id           = $1
                 AND  bsl.statement_id = bs.id
                 AND  bs.agency_id     = $2
               RETURNING bsl.id, bsl.statement_id, bsl.value_date, bsl.description,
                         bsl.amount, bsl.reference, bsl.matched_entry_id, bsl.created_at"#,
        )
        .bind(cmd.line_id)
        .bind(cmd.agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("bank statement line {}", cmd.line_id)))?;

        Ok(line_from_row(row))
    }

    async fn get_reconciliation_report(
        &self,
        agency_id: Uuid,
        statement_id: Uuid,
    ) -> Result<ReconciliationReport, AppError> {
        let stmt = self
            .find_statement_by_id(agency_id, statement_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("bank statement {statement_id}")))?;

        let matched_total: Decimal = stmt
            .lines
            .iter()
            .filter(|l| l.status == ReconciliationStatus::Matched)
            .map(|l| l.amount)
            .sum();

        let unmatched_lines: Vec<BankStatementLine> = stmt
            .lines
            .iter()
            .filter(|l| l.status == ReconciliationStatus::Unmatched)
            .cloned()
            .collect();

        let unmatched_total: Decimal = unmatched_lines.iter().map(|l| l.amount).sum();
        let matched_count = stmt
            .lines
            .iter()
            .filter(|l| l.status == ReconciliationStatus::Matched)
            .count() as i64;
        let unmatched_count = unmatched_lines.len() as i64;

        Ok(ReconciliationReport {
            statement_id: stmt.id,
            bank_name: stmt.bank_name,
            account_number: stmt.account_number,
            statement_date: stmt.statement_date,
            opening_balance: stmt.opening_balance,
            closing_balance: stmt.closing_balance,
            matched_total,
            unmatched_total,
            matched_count,
            unmatched_count,
            unmatched_lines,
        })
    }
}
