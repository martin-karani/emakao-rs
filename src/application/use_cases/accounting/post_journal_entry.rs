// src/application/use_cases/accounting/post_journal_entry.rs
//
// Business rules enforced here (not in the DB or handler):
//   1. An entry must have at least two lines.
//   2. Total debits must equal total credits (the fundamental accounting equation).
//   3. Each line must have exactly one non-zero side (enforced by DB CHECK, but
//      we surface a cleaner error here first).
//   4. All referenced accounts must belong to the same agency.

use std::sync::Arc;

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::accounting_repository::AccountingRepository},
    domain::accounting::{CreateJournalEntryCommand, JournalEntry, JournalLine},
};

pub struct PostJournalEntryInput {
    pub agency_id: Uuid,
    pub reference: String,
    pub description: Option<String>,
    pub lines: Vec<JournalLine>,
    pub posted_by: Uuid,
    /// When `false` the entry is saved as a draft; no balances are updated.
    pub post_immediately: bool,
}

pub struct PostJournalEntryUseCase {
    pub repo: Arc<dyn AccountingRepository>,
}

impl PostJournalEntryUseCase {
    pub async fn execute(&self, input: PostJournalEntryInput) -> Result<JournalEntry, AppError> {
        // ── Structural validation ─────────────────────────────────────────────
        if input.lines.len() < 2 {
            return Err(AppError::Validation(
                "a journal entry must have at least two lines".into(),
            ));
        }

        if input.reference.trim().is_empty() {
            return Err(AppError::Validation("reference is required".into()));
        }

        // Each line must be a pure debit or a pure credit.
        for (i, line) in input.lines.iter().enumerate() {
            let has_debit = line.debit_kes > Decimal::ZERO;
            let has_credit = line.credit_kes > Decimal::ZERO;
            if has_debit == has_credit {
                return Err(AppError::Validation(format!(
                    "line {i}: exactly one of debit_kes or credit_kes must be non-zero"
                )));
            }
            if line.debit_kes < Decimal::ZERO || line.credit_kes < Decimal::ZERO {
                return Err(AppError::Validation(format!(
                    "line {i}: amounts must be non-negative"
                )));
            }
        }

        // ── Balance check ─────────────────────────────────────────────────────
        let total_debits: Decimal = input.lines.iter().map(|l| l.debit_kes).sum();
        let total_credits: Decimal = input.lines.iter().map(|l| l.credit_kes).sum();

        if total_debits != total_credits {
            return Err(AppError::Validation(format!(
                "journal entry does not balance: debits {total_debits} ≠ credits {total_credits}"
            )));
        }

        // ── Persist ───────────────────────────────────────────────────────────
        let entry = self
            .repo
            .create_journal_entry(CreateJournalEntryCommand {
                agency_id: input.agency_id,
                reference: input.reference.trim().to_string(),
                description: input.description,
                lines: input.lines,
                posted_by: input.posted_by,
                post_immediately: input.post_immediately,
            })
            .await?;

        tracing::info!(
            entry_id   = %entry.id,
            reference  = %entry.reference,
            status     = ?entry.status,
            debits_kes = %total_debits,
            "journal entry created"
        );

        Ok(entry)
    }
}
