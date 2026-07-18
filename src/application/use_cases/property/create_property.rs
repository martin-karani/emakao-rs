use std::{collections::HashSet, sync::Arc};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            openfga_port::OpenFgaPort, property_repository::PropertyRepository,
            unit_repository::UnitRepository,
        },
    },
    domain::{
        enums::PropertyType,
        errors::DomainError,
        property::{
            slugify, unique_slug, CreatePropertyCommand, CreateUnitCommand, Property,
            PropertyConfig, PropertyDocument, PropertyPolicies, UnitType,
        },
    },
};

pub struct CreatePropertyUseCase {
    pub repo: Arc<dyn PropertyRepository>,
    pub unit_repo: Arc<dyn UnitRepository>,
    pub openfga: Arc<dyn OpenFgaPort>,
    pub invite_caretaker:
        crate::application::use_cases::maintenance::invite_caretaker::InviteCaretakerUseCase,
}

impl CreatePropertyUseCase {
    pub fn new(
        repo: Arc<dyn PropertyRepository>,
        unit_repo: Arc<dyn UnitRepository>,
        openfga: Arc<dyn OpenFgaPort>,
        invite_caretaker: crate::application::use_cases::maintenance::invite_caretaker::InviteCaretakerUseCase,
    ) -> Self {
        Self {
            repo,
            unit_repo,
            openfga,
            invite_caretaker,
        }
    }
}

pub struct CreatePropertyInput {
    pub agency_id: Uuid,
    pub fga_store_id: Option<String>,
    pub created_by: Uuid,
    pub name: String,
    pub address: String,
    pub city: String,
    pub property_type: PropertyType,
    pub config: PropertyConfig,
    pub unit_types: Vec<UnitTypeInput>,
    pub photos: Vec<String>,
    pub documents: Vec<PropertyDocument>,
    /// Optional override for the work-order prefix (e.g. `"PARK"`).
    /// When `None` the repo derives one from the property name automatically.
    /// Must be 2–8 uppercase ASCII letters/digits when supplied.
    pub work_order_prefix: Option<String>,
    pub owner_ids: Vec<Uuid>,
    pub agent_ids: Vec<Uuid>,
    pub new_caretakers: Vec<NewCaretakerInput>,
    pub portal_base_url: String,
    pub agency_name: Option<String>,
    pub policies: Option<PropertyPolicies>,
}

pub struct UnitTypeInput {
    pub name: String,
    pub unit_type: Option<String>,
    pub bedrooms: i16,
    pub bathrooms: i16,
    pub size_sqm: Option<f64>,
    pub photos: Option<Vec<String>>,
    pub base_rent: Option<rust_decimal::Decimal>,
    pub base_deposit: Option<rust_decimal::Decimal>,
    pub quantity: i32,
    pub unit_numbers: Option<Vec<String>>,
}

pub struct NewCaretakerInput {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

impl CreatePropertyUseCase {
    pub async fn execute(&self, input: CreatePropertyInput) -> Result<Property, AppError> {
        // ── Validation ────────────────────────────────────────────────────────

        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::PropertyNameEmpty.into());
        }

        // Check for existing property with the same name in agency
        let existing_property = self.repo.find_by_name(input.agency_id, &name).await?;
        if existing_property.is_some() {
            return Err(AppError::Validation("A property with this name already exists in your agency".into()));
        }

        let address = input.address.trim().to_string();
        if address.is_empty() {
            return Err(AppError::Validation("address must not be empty".into()));
        }

        let city = input.city.trim().to_string();
        let base_slug = slugify(&name);
        let existing_slugs = self.repo.list_slugs_for_agency(input.agency_id).await?;
        let slug = unique_slug(&base_slug, &existing_slugs);

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

        let unit_types: Vec<UnitType> = input
            .unit_types
            .into_iter()
            .map(|ut| UnitType {
                id: Uuid::new_v4(),
                name: ut.name.clone(),
                unit_type: ut.unit_type.clone(),
                bedrooms: ut.bedrooms,
                bathrooms: ut.bathrooms,
                photos: ut.photos.clone(),
                size_sqm: ut.size_sqm,
                base_rent: ut.base_rent,
                base_deposit: ut.base_deposit,
                quantity: ut.quantity,
                unit_numbers: ut.unit_numbers,
            })
            .collect();

        let property = self
            .repo
            .create(CreatePropertyCommand {
                agency_id: input.agency_id,
                created_by: input.created_by,
                slug,
                name,
                address,
                city,
                property_type: input.property_type,
                config: input.config,
                unit_types: unit_types.clone(),
                photos: input.photos,
                documents: input.documents,
                work_order_prefix,
                country_code: "KE".to_string(),
                owner_ids: input.owner_ids,
                agent_ids: input.agent_ids,
                policies: input.policies,
            })
            .await?;

        // ── Generate Units ───────────────────────────────────────────────────

        let mut unit_cmds = Vec::new();
        let mut unit_seq = 1;
        let mut used_unit_numbers = HashSet::new();

        for ut in &unit_types {
            for i in 0..ut.quantity {
                let requested_unit_number = ut
                    .unit_numbers
                    .as_ref()
                    .and_then(|nums| nums.get(i as usize))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());

                let unit_number = match requested_unit_number {
                    Some(unit_number)
                        if !used_unit_numbers.contains(&unit_number) =>
                    {
                        unit_number
                    }
                    Some(unit_number) if unit_number.parse::<u32>().is_ok() => {
                        next_generated_unit_number(&used_unit_numbers, &mut unit_seq)
                    }
                    Some(unit_number) => {
                        return Err(AppError::Validation(format!(
                            "duplicate unit number '{unit_number}' provided while creating the property"
                        )));
                    }
                    None => next_generated_unit_number(&used_unit_numbers, &mut unit_seq),
                };

                used_unit_numbers.insert(unit_number.clone());

                unit_cmds.push(CreateUnitCommand {
                    property_id: property.id,
                    unit_type_id: Some(ut.id),
                    unit_number,
                    floor: None,
                    size_sqm: None,
                    bedrooms: Some(ut.bedrooms),
                    bathrooms: Some(ut.bathrooms),
                    rent_amount_kes: ut.base_rent.unwrap_or_default(),
                    deposit_kes: ut.base_deposit.unwrap_or_default(),
                    description: Some(format!("Type: {}", ut.name)),
                });
            }
        }

        if !unit_cmds.is_empty() {
            let _ = self.unit_repo.create_batch(unit_cmds).await?;
        }

        // ── OpenFGA Tuples ───────────────────────────────────────────────────
        //
        // 1. `agency:{id}` — parent_agency — `property:{id}`
        // 2. `user:{created_by}` — manager — `property:{id}`
        if let Some(ref store_id) = input.fga_store_id {
            let property_obj = format!("property:{}", property.id);
            let agency_obj = format!("agency:{}", input.agency_id);
            let creator_user = format!("user:{}", input.created_by);

            // Parent link
            let _ = self
                .openfga
                .write_tuple(store_id, &agency_obj, "parent_agency", &property_obj)
                .await;

            // Creator is manager
            let _ = self
                .openfga
                .write_tuple(store_id, &creator_user, "manager", &property_obj)
                .await;

            tracing::info!(
                property_id = %property.id,
                store_id = %store_id,
                "OpenFGA tuples written for new property"
            );
        }

        // ── Invite Caretakers ────────────────────────────────────────────────
        for ct in input.new_caretakers {
            let _ = self
                .invite_caretaker
                .execute(crate::application::use_cases::maintenance::invite_caretaker::InviteCaretakerInput {
                    agency_id: input.agency_id,
                    property_id: property.id,
                    created_by: input.created_by,
                    first_name: ct.first_name,
                    last_name: ct.last_name,
                    email: ct.email,
                    phone: ct.phone,
                    portal_base_url: input.portal_base_url.clone(),
                    agency_name: input.agency_name.clone(),
                })
                .await;
        }

        tracing::info!(
            property_id          = %property.id,
            work_order_prefix    = %property.maintenance.work_order_prefix,
            "property created"
        );

        Ok(property)
    }
}

fn next_generated_unit_number(used_unit_numbers: &HashSet<String>, unit_seq: &mut i32) -> String {
    loop {
        let candidate = unit_seq.to_string();
        *unit_seq += 1;

        if !used_unit_numbers.contains(&candidate) {
            return candidate;
        }
    }
}
