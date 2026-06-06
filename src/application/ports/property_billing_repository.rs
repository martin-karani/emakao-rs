// src/application/ports/property_billing_repository.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::property_billing::{PropertyBillingSettings, PropertyBillingSummary},
};

#[async_trait]
pub trait PropertyBillingRepository: Send + Sync + 'static {
    async fn get_by_property_id(
        &self,
        property_id: Uuid,
    ) -> Result<Option<PropertyBillingSettings>, AppError>;
    async fn upsert(&self, settings: PropertyBillingSettings) -> Result<(), AppError>;
    async fn get_agency_summary(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<PropertyBillingSummary>, AppError>;
}
