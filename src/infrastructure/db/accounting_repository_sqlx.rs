use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::accounting_repository::AccountingRepository},
    domain::accounting::{
        system_accounts, Account, AccountFilter, AccountType, CreateAccountCommand,
        CreateJournalEntryCommand, JournalEntry, JournalEntryFilter, JournalEntryStatus,
        JournalLine, TrialBalance, TrialBalanceLine, VatReport, VatReportFilter, VatReportLine,
        VoidJournalEntryCommand,
    },
};

pub struct PgAccountingRepo {
    pool: PgPool,
}

impl PgAccountingRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Row types ─────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
    agency_id: Uuid,
    code: String,
    name: String,
    account_type: String,
    balance: Decimal,
    is_system: bool,
    vat_applicable: bool,
    vat_rate: Option<Decimal>,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

impl TryFrom<AccountRow> for Account {
    type Error = AppError;
    fn try_from(r: AccountRow) -> Result<Self, Self::Error> {
        let account_type = AccountType::from_str(&r.account_type).ok_or_else(|| {
            AppError::InternalServer(format!("unknown account type: {}", r.account_type))
        })?;
        Ok(Account {
            id: r.id,
            agency_id: r.agency_id,
            code: r.code,
            name: r.name,
            account_type,
            balance: r.balance,
            is_system: r.is_system,
            vat_applicable: r.vat_applicable,
            vat_rate: r.vat_rate,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    agency_id: Uuid,
    reference: String,
    description: Option<String>,
    status: String,
    posted_by: Uuid,
    posted_at: time::OffsetDateTime,
    created_at: time::OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    account_id: Uuid,
    debit_kes: Decimal,
    credit_kes: Decimal,
    description: Option<String>,
}

fn entry_from_row(row: EntryRow, lines: Vec<JournalLine>) -> Result<JournalEntry, AppError> {
    let status = JournalEntryStatus::from_str(&row.status).ok_or_else(|| {
        AppError::InternalServer(format!("unknown journal entry status: {}", row.status))
    })?;
    Ok(JournalEntry {
        id: row.id,
        agency_id: row.agency_id,
        reference: row.reference,
        description: row.description,
        status,
        lines,
        posted_by: row.posted_by,
        posted_at: row.posted_at,
        created_at: row.created_at,
    })
}

async fn load_lines(pool: &PgPool, entry_id: Uuid) -> Result<Vec<JournalLine>, AppError> {
    let rows = sqlx::query_as::<_, LineRow>(
        r#"SELECT account_id, debit_kes, credit_kes, description
           FROM   journal_lines
           WHERE  journal_entry_id = $1
           ORDER BY created_at"#,
    )
    .bind(entry_id)
    .fetch_all(pool)
    .await
    .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| JournalLine {
            account_id: r.account_id,
            debit_kes: r.debit_kes,
            credit_kes: r.credit_kes,
            description: r.description,
        })
        .collect())
}

// ── Repository impl ───────────────────────────────────────────────────────────

#[async_trait]
impl AccountingRepository for PgAccountingRepo {
    // ── Accounts ──────────────────────────────────────────────────────────────

    async fn list_accounts(&self, filter: AccountFilter) -> Result<Vec<Account>, AppError> {
        let type_str = filter.account_type.as_ref().map(|t| t.as_str());
        let rows = sqlx::query_as::<_, AccountRow>(
            r#"SELECT id, agency_id, code, name, account_type, balance, is_system,
                      vat_applicable, vat_rate, created_at, updated_at
               FROM   accounts
               WHERE  agency_id = $1
                 AND ($2::text IS NULL OR account_type = $2::text::account_category)
               ORDER BY code
               LIMIT $3 OFFSET $4"#,
        )
        .bind(filter.agency_id)
        .bind(type_str)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        rows.into_iter().map(Account::try_from).collect()
    }

    async fn find_account_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Account>, AppError> {
        let row = sqlx::query_as::<_, AccountRow>(
            r#"SELECT id, agency_id, code, name, account_type, balance, is_system,
                      vat_applicable, vat_rate, created_at, updated_at
               FROM   accounts
               WHERE  id = $1 AND agency_id = $2"#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        row.map(Account::try_from).transpose()
    }

    async fn create_account(&self, cmd: CreateAccountCommand) -> Result<Account, AppError> {
        let row = sqlx::query_as::<_, AccountRow>(
            r#"INSERT INTO accounts
                   (id, agency_id, code, name, account_type, is_system, vat_applicable, vat_rate)
               VALUES (uuidv7(), $1, $2, $3, $4::text::account_category, $5, $6, $7)
               RETURNING id, agency_id, code, name, account_type, balance, is_system,
                         vat_applicable, vat_rate, created_at, updated_at"#,
        )
        .bind(cmd.agency_id)
        .bind(&cmd.code)
        .bind(&cmd.name)
        .bind(cmd.account_type.as_str())
        .bind(cmd.is_system)
        .bind(cmd.vat_applicable)
        .bind(cmd.vat_rate)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref dbe) = e {
                if dbe.constraint() == Some("accounts_agency_id_code_key") {
                    return AppError::Conflict(format!(
                        "account code '{}' already exists",
                        cmd.code
                    ));
                }
            }
            AppError::InternalServer(e.to_string())
        })?;

        Account::try_from(row)
    }

    async fn delete_account(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let acct = self
            .find_account_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("account {id}")))?;

        if acct.is_system {
            return Err(AppError::Validation(
                "system accounts cannot be deleted".into(),
            ));
        }

        sqlx::query("DELETE FROM accounts WHERE id = $1 AND agency_id = $2")
            .bind(id)
            .bind(agency_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(ref dbe) = e {
                    if dbe.constraint().is_some() {
                        return AppError::Conflict(
                            "account has journal lines and cannot be deleted".into(),
                        );
                    }
                }
                AppError::InternalServer(e.to_string())
            })?;

        Ok(())
    }

    async fn seed_system_accounts(&self, agency_id: Uuid) -> Result<(), AppError> {
        for seed in system_accounts() {
            // Use INSERT … ON CONFLICT DO NOTHING so re-running provisioning is safe.
            sqlx::query(
                r#"INSERT INTO accounts
                       (id, agency_id, code, name, account_type, is_system, vat_applicable, vat_rate)
                   VALUES (uuidv7(), $1, $2, $3, $4::text::account_category, true, $5, $6)
                   ON CONFLICT (agency_id, code) DO NOTHING"#,
            )
            .bind(agency_id)
            .bind(seed.code)
            .bind(seed.name)
            .bind(seed.account_type.as_str())
            .bind(seed.vat_applicable)
            .bind(seed.vat_rate)
            .execute(&self.pool)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        }

        tracing::info!(agency_id = %agency_id, "system chart of accounts seeded");
        Ok(())
    }

    // ── Journal entries ───────────────────────────────────────────────────────

    async fn list_journal_entries(
        &self,
        filter: JournalEntryFilter,
    ) -> Result<Vec<JournalEntry>, AppError> {
        let status_str = filter.status.as_ref().map(|s| s.as_str());
        let rows = sqlx::query_as::<_, EntryRow>(
            r#"SELECT id, agency_id, reference, description, status, posted_by,
                      posted_at, created_at
               FROM   journal_entries
               WHERE  agency_id = $1
                 AND ($2::text IS NULL OR status = $2::text::journal_entry_status)
               ORDER BY posted_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(filter.agency_id)
        .bind(status_str)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let lines = load_lines(&self.pool, id).await?;
            entries.push(entry_from_row(row, lines)?);
        }
        Ok(entries)
    }

    async fn find_journal_entry_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<JournalEntry>, AppError> {
        let row = sqlx::query_as::<_, EntryRow>(
            r#"SELECT id, agency_id, reference, description, status, posted_by,
                      posted_at, created_at
               FROM   journal_entries
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
                Ok(Some(entry_from_row(r, lines)?))
            }
        }
    }

    async fn create_journal_entry(
        &self,
        cmd: CreateJournalEntryCommand,
    ) -> Result<JournalEntry, AppError> {
        let status = if cmd.post_immediately {
            "posted"
        } else {
            "draft"
        };
        let mut tx: Transaction<'_, Postgres> = self
            .pool
            .begin()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let entry_row = sqlx::query_as::<_, EntryRow>(
            r#"INSERT INTO journal_entries
                   (id, agency_id, reference, description, status, posted_by)
               VALUES (uuidv7(), $1, $2, $3, $4::text::journal_entry_status, $5)
               RETURNING id, agency_id, reference, description, status, posted_by,
                         posted_at, created_at"#,
        )
        .bind(cmd.agency_id)
        .bind(cmd.reference)
        .bind(cmd.description)
        .bind(status)
        .bind(cmd.posted_by)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let entry_id = entry_row.id;

        for line in &cmd.lines {
            sqlx::query(
                r#"INSERT INTO journal_lines
                       (id, journal_entry_id, account_id, debit_kes, credit_kes, description)
                   VALUES (uuidv7(), $1, $2, $3, $4, $5)"#,
            )
            .bind(entry_id)
            .bind(line.account_id)
            .bind(line.debit_kes)
            .bind(line.credit_kes)
            .bind(&line.description)
            .execute(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

            if cmd.post_immediately {
                let net = line.debit_kes - line.credit_kes;
                sqlx::query(
                    "UPDATE accounts SET balance = balance + $1, updated_at = now()
                     WHERE id = $2 AND agency_id = $3",
                )
                .bind(net)
                .bind(line.account_id)
                .bind(cmd.agency_id)
                .execute(&mut *tx)
                .await
                .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
            }
        }

        tx.commit()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let lines = cmd
            .lines
            .into_iter()
            .map(|l| JournalLine {
                account_id: l.account_id,
                debit_kes: l.debit_kes,
                credit_kes: l.credit_kes,
                description: l.description,
            })
            .collect();

        entry_from_row(entry_row, lines)
    }

    async fn void_journal_entry(
        &self,
        cmd: VoidJournalEntryCommand,
    ) -> Result<JournalEntry, AppError> {
        let mut tx: Transaction<'_, Postgres> = self
            .pool
            .begin()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let entry = self
            .find_journal_entry_by_id(cmd.agency_id, cmd.entry_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("journal entry {}", cmd.entry_id)))?;

        if entry.status == JournalEntryStatus::Voided {
            return Err(AppError::Conflict("entry is already voided".into()));
        }

        if entry.status == JournalEntryStatus::Posted {
            for line in &entry.lines {
                let reversal = line.credit_kes - line.debit_kes;
                sqlx::query(
                    "UPDATE accounts SET balance = balance + $1, updated_at = now()
                     WHERE id = $2 AND agency_id = $3",
                )
                .bind(reversal)
                .bind(line.account_id)
                .bind(cmd.agency_id)
                .execute(&mut *tx)
                .await
                .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
            }
        }

        let updated = sqlx::query_as::<_, EntryRow>(
            r#"UPDATE journal_entries
               SET status = 'voided'::journal_entry_status
               WHERE id = $1 AND agency_id = $2
               RETURNING id, agency_id, reference, description, status, posted_by,
                         posted_at, created_at"#,
        )
        .bind(cmd.entry_id)
        .bind(cmd.agency_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        entry_from_row(updated, entry.lines)
    }

    // ── Reports ───────────────────────────────────────────────────────────────

    async fn get_trial_balance(&self, agency_id: Uuid) -> Result<TrialBalance, AppError> {
        #[derive(sqlx::FromRow)]
        struct TbRow {
            account_id: Uuid,
            code: String,
            name: String,
            account_type: String,
            total_debits: Option<Decimal>,
            total_credits: Option<Decimal>,
        }

        let rows = sqlx::query_as::<_, TbRow>(
            r#"SELECT a.id AS account_id, a.code, a.name, a.account_type,
                      COALESCE(SUM(jl.debit_kes),  0) AS total_debits,
                      COALESCE(SUM(jl.credit_kes), 0) AS total_credits
               FROM   accounts a
               LEFT JOIN journal_lines  jl ON jl.account_id = a.id
               LEFT JOIN journal_entries je ON je.id = jl.journal_entry_id
                  AND je.status = 'posted' AND je.agency_id = $1
               WHERE  a.agency_id = $1
               GROUP  BY a.id, a.code, a.name, a.account_type
               ORDER  BY a.code"#,
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let mut lines = Vec::with_capacity(rows.len());
        let mut grand_debits = Decimal::ZERO;
        let mut grand_credits = Decimal::ZERO;

        for row in rows {
            let account_type = AccountType::from_str(&row.account_type).ok_or_else(|| {
                AppError::InternalServer(format!("unknown type: {}", row.account_type))
            })?;
            let total_debits = row.total_debits.unwrap_or(Decimal::ZERO);
            let total_credits = row.total_credits.unwrap_or(Decimal::ZERO);
            let net_balance = total_debits - total_credits;
            grand_debits += total_debits;
            grand_credits += total_credits;
            lines.push(TrialBalanceLine {
                account_id: row.account_id,
                code: row.code,
                name: row.name,
                account_type,
                total_debits,
                total_credits,
                net_balance,
            });
        }

        Ok(TrialBalance {
            is_balanced: grand_debits == grand_credits,
            total_debits: grand_debits,
            total_credits: grand_credits,
            lines,
        })
    }

    async fn get_vat_report(&self, filter: VatReportFilter) -> Result<VatReport, AppError> {
        #[derive(sqlx::FromRow)]
        struct VatRow {
            account_id: Uuid,
            code: String,
            name: String,
            account_type: String,
            vat_rate: Option<Decimal>,
            total_debits: Option<Decimal>,
            total_credits: Option<Decimal>,
        }

        let rows = sqlx::query_as::<_, VatRow>(
            r#"SELECT
                 a.id          AS account_id,
                 a.code,
                 a.name,
                 a.account_type,
                 a.vat_rate,
                 COALESCE(SUM(jl.debit_kes),  0) AS total_debits,
                 COALESCE(SUM(jl.credit_kes), 0) AS total_credits
               FROM   accounts a
               JOIN   journal_lines jl  ON jl.account_id = a.id
               JOIN   journal_entries je ON je.id = jl.journal_entry_id
                 AND  je.status    = 'posted'
                 AND  je.agency_id = $1
                 AND  je.posted_at >= $2
                 AND  je.posted_at <  $3 + INTERVAL '1 day'
               WHERE  a.agency_id     = $1
                 AND  a.vat_applicable = true
               GROUP  BY a.id, a.code, a.name, a.account_type, a.vat_rate
               ORDER  BY a.code"#,
        )
        .bind(filter.agency_id)
        .bind(filter.period_start)
        .bind(filter.period_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let mut vat_lines: Vec<VatReportLine> = Vec::new();
        let mut output_vat = Decimal::ZERO;
        let mut input_vat = Decimal::ZERO;

        for row in rows {
            let account_type = AccountType::from_str(&row.account_type).ok_or_else(|| {
                AppError::InternalServer(format!("unknown type: {}", row.account_type))
            })?;
            let vat_rate = row.vat_rate.unwrap_or(Decimal::new(1600, 4));
            let debits = row.total_debits.unwrap_or(Decimal::ZERO);
            let credits = row.total_credits.unwrap_or(Decimal::ZERO);

            // Net for revenue = credits - debits (credit-normal).
            // Net for expenses = debits - credits (debit-normal).
            let net = match account_type {
                AccountType::Revenue => credits - debits,
                _ => debits - credits,
            };

            let vat_amount = net * vat_rate;

            match account_type {
                AccountType::Revenue => output_vat += vat_amount,
                AccountType::Expense => input_vat += vat_amount,
                _ => {}
            }

            vat_lines.push(VatReportLine {
                account_id: row.account_id,
                code: row.code,
                name: row.name,
                account_type,
                net_amount_kes: net,
                vat_amount_kes: vat_amount,
                vat_rate,
            });
        }

        Ok(VatReport {
            period_start: filter.period_start,
            period_end: filter.period_end,
            output_vat_kes: output_vat,
            input_vat_kes: input_vat,
            net_vat_payable_kes: output_vat - input_vat,
            lines: vat_lines,
        })
    }
}
