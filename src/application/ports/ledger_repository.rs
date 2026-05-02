use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::ledger::{BalanceSummary, CreateLedgerEntryCommand, LedgerEntry},
};

#[async_trait]
pub trait LedgerRepository: Send + Sync + 'static {
    async fn create(
        &self,
        cmd: CreateLedgerEntryCommand,
    ) -> Result<LedgerEntry, AppError>;

    async fn find_by_agreement(
        &self,
        agreement_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LedgerEntry>, AppError>;

    async fn balance_for_agreement(
        &self,
        agreement_id: Uuid,
    ) -> Result<BalanceSummary, AppError>;
}