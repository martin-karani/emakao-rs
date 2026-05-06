use crate::{
    application::{errors::AppError, ports::utility_repository::UtilityRepository},
    domain::{enums::UtilityBillStatus, utility::UtilityBill},
};
use std::sync::Arc;
use uuid::Uuid;

pub struct ListBillsUseCase {
    pub repo: Arc<dyn UtilityRepository>,
}

impl ListBillsUseCase {
    pub fn new(repo: Arc<dyn UtilityRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        unit_id: Uuid,
        status: Option<UtilityBillStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UtilityBill>, AppError> {
        self.repo
            .find_bills_by_unit(unit_id, status, limit, offset)
            .await
    }
}
