use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::property_repository::PropertyRepository},
    domain::{
        enums::PropertyType,
        errors::DomainError,
        property::{CreatePropertyCommand, Property, PropertyConfig},
    },
};

pub struct CreatePropertyUseCase {
    pub repo: Arc<dyn PropertyRepository>,
}

impl CreatePropertyUseCase {
    pub fn new(repo: Arc<dyn PropertyRepository>) -> Self {
        Self { repo }
    }
}

pub struct CreatePropertyInput {
    pub agency_id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub address: String,
    pub city: String,
    pub property_type: PropertyType,
    pub config: PropertyConfig,
    /// Optional override for the work-order prefix (e.g. `"PARK"`).
    /// When `None` the repo derives one from the property name automatically.
    /// Must be 2–8 uppercase ASCII letters/digits when supplied.
    pub work_order_prefix: Option<String>,
}

impl CreatePropertyUseCase {
    pub async fn execute(&self, input: CreatePropertyInput) -> Result<Property, AppError> {
        // ── Validation ────────────────────────────────────────────────────────

        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::PropertyNameEmpty.into());
        }

        let address = input.address.trim().to_string();
        if address.is_empty() {
            return Err(AppError::Validation("address must not be empty".into()));
        }

        let city = input.city.trim().to_string();

        // Validate the custom prefix when the caller supplied one.
        let work_order_prefix = input
            .work_order_prefix
            .map(|p| p.trim().to_ascii_uppercase())
            .filter(|p| !p.is_empty());

        if let Some(ref prefix) = work_order_prefix {
            let valid_chars = prefix
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
            let valid_len = (2..=8).contains(&prefix.len());
            let starts_alpha = prefix
                .chars()
                .next()
                .map(|c| c.is_ascii_uppercase())
                .unwrap_or(false);

            if !valid_chars || !valid_len || !starts_alpha {
                return Err(AppError::Validation(
                    "work_order_prefix must be 2–8 characters, \
                     start with a letter, and contain only uppercase letters or digits \
                     (e.g. \"PARK\", \"MG1\")"
                        .into(),
                ));
            }
        }

        // ── Persist ───────────────────────────────────────────────────────────

        let property = self
            .repo
            .create(CreatePropertyCommand {
                agency_id: input.agency_id,
                created_by: input.created_by,
                name,
                address,
                city,
                property_type: input.property_type,
                config: input.config,
                work_order_prefix,
            })
            .await?;

        tracing::info!(
            property_id          = %property.id,
            work_order_prefix    = %property.work_order_prefix,
            "property created"
        );

        Ok(property)
    }
}
