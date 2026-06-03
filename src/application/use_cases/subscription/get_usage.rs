use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    domain::subscription::{AgencyEntitlements, LimitKey, UsageSummary},
    infrastructure::cache::subscription_cache::SubscriptionCache,
};

use super::check_limit::check_limit_full;

pub struct GetUsageUseCase {
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
}

impl GetUsageUseCase {
    pub fn new(repo: Arc<dyn SubscriptionRepository>, cache: Arc<SubscriptionCache>) -> Self {
        Self { repo, cache }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        entitlements: &AgencyEntitlements,
    ) -> Result<UsageSummary, AppError> {
        // Fetch counts in parallel (Mapping "properties" table to MaxBranches concept)
        let (branches, units, storage_mb) = tokio::try_join!(
            self.repo.count_tenant_rows(agency_id, "properties"),
            self.repo.count_tenant_rows(agency_id, "units"),
            self.get_storage_mb(agency_id),
        )?;

        let sms_this_month = self.cache.get_monthly_counter(agency_id, LimitKey::MaxSmsPerMonth).await;
        let wa_this_month = self.cache.get_monthly_counter(agency_id, LimitKey::MaxWhatsappPerMonth).await;

        Ok(UsageSummary {
            branches: check_limit_full(entitlements, &LimitKey::MaxBranches, branches, None),
            units: check_limit_full(entitlements, &LimitKey::MaxUnits, units, None),
            storage: check_limit_full(entitlements, &LimitKey::MaxStorageMb, storage_mb, None),
            sms: check_limit_full(entitlements, &LimitKey::MaxSmsPerMonth, sms_this_month, None),
            whatsapp: check_limit_full(entitlements, &LimitKey::MaxWhatsappPerMonth, wa_this_month, None),
        })
    }

    async fn get_storage_mb(&self, agency_id: Uuid) -> Result<i32, AppError> {
        // Storage bytes tracked separately; fall back to 0 if not tracked yet.
        let bytes = self.cache.get_usage_bytes(agency_id).await;
        Ok((bytes as f64 / 1_048_576.0).ceil() as i32)
    }
}
