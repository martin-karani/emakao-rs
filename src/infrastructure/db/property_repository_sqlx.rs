use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::property_repository::{PropertyFilter, PropertyRepository},
    },
    domain::{
        enums::PropertyType,
        property::{CreatePropertyCommand, Property, PropertyConfig, UpdatePropertyCommand},
        work_order_code::{candidate_prefix, unique_prefix},
    },
};

pub struct PgPropertyRepo {
    pool: PgPool,
}

impl PgPropertyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgPropertyRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Internal row type ─────────────────────────────────────────────────────────
// Includes every column returned by INSERT … RETURNING and SELECT queries.
// `work_order_prefix` and `work_order_seq` are internal implementation details
// not surfaced in the `Property` domain model, but sqlx requires the struct to
// cover all columns in the RETURNING clause.

#[derive(sqlx::FromRow)]
struct PropertyRow {
    id: Uuid,
    agency_id: Uuid,
    name: String,
    address: String,
    city: String,
    country_code: String,
    property_type: String,
    config: sqlx::types::Json<serde_json::Value>,
    // Internal work-order sequencing — not on the domain model
    work_order_prefix: String,
    work_order_seq: i32,
    created_by: Uuid,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

impl TryFrom<PropertyRow> for Property {
    type Error = AppError;

    fn try_from(row: PropertyRow) -> Result<Self, Self::Error> {
        let property_type: PropertyType =
            serde_json::from_value(serde_json::Value::String(row.property_type))
                .map_err(|e| AppError::ExternalService(e.to_string()))?;

        let config: PropertyConfig = serde_json::from_value(row.config.0)
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        Ok(Property {
            id: row.id,
            agency_id: row.agency_id,
            name: row.name,
            address: row.address,
            city: row.city,
            country_code: row.country_code,
            property_type,
            config,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

// ── Repository ────────────────────────────────────────────────────────────────

#[async_trait]
impl PropertyRepository for PgPropertyRepo {
    async fn find_all(&self, filter: PropertyFilter) -> Result<Vec<Property>, AppError> {
        let rows = sqlx::query_as!(
            PropertyRow,
            r#"
            SELECT
                id, agency_id, name, address, city, country_code,
                property_type,
                config AS "config: sqlx::types::Json<serde_json::Value>",
                work_order_prefix, work_order_seq,
                created_by, created_at, updated_at
            FROM properties
            WHERE agency_id = $1
              AND ($2::text IS NULL OR property_type = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            filter.agency_id,
            filter.property_type,
            filter.limit,
            filter.offset
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Property::try_from).collect()
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Property>, AppError> {
        let row = sqlx::query_as!(
            PropertyRow,
            r#"
            SELECT
                id, agency_id, name, address, city, country_code,
                property_type,
                config AS "config: sqlx::types::Json<serde_json::Value>",
                work_order_prefix, work_order_seq,
                created_by, created_at, updated_at
            FROM properties
            WHERE id = $1 AND agency_id = $2
            "#,
            id,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(Property::try_from).transpose()
    }

    async fn create(&self, cmd: CreatePropertyCommand) -> Result<Property, AppError> {
        // ── 1. Determine the prefix ────────────────────────────────────────────
        let base = match cmd.work_order_prefix {
            Some(ref p) if !p.trim().is_empty() => p.trim().to_ascii_uppercase(),
            _ => candidate_prefix(&cmd.name),
        };

        if !base
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            || base.len() < 2
            || base.len() > 8
        {
            return Err(AppError::Validation(
                "work_order_prefix must be 2-8 uppercase letters/digits".into(),
            ));
        }

        // ── 2. Fetch existing prefixes in this agency schema ───────────────────
        let taken: HashSet<String> = sqlx::query_scalar!(
            "SELECT work_order_prefix FROM properties WHERE work_order_prefix LIKE $1",
            format!("{}%", &base[..base.len().min(6)])
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect();

        let prefix = unique_prefix(&base, &taken);

        // ── 3. Insert ──────────────────────────────────────────────────────────
        // country_code defaults to "KE" (Kenyan market) when not supplied.
        let country_code = cmd
            .country_code
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("KE")
            .to_ascii_uppercase();

        let row = sqlx::query_as!(
            PropertyRow,
            r#"
            INSERT INTO properties (
                id, agency_id, name, address, city, country_code,
                property_type, config, work_order_prefix, created_by
            )
            VALUES (
                uuidv7(), $1::uuid, $2::text, $3::text, $4::text, $5::text,
                $6::text, $7::jsonb, $8::varchar, $9::uuid
            )
            RETURNING
                id, agency_id, name, address, city, country_code,
                property_type,
                config AS "config: sqlx::types::Json<serde_json::Value>",
                work_order_prefix, work_order_seq,
                created_by, created_at, updated_at
            "#,
            cmd.agency_id,                                         // $1
            cmd.name,                                              // $2
            cmd.address,                                           // $3
            cmd.city,                                              // $4
            country_code,                                          // $5 — FIX
            property_type_str(&cmd.property_type),                 // $6
            serde_json::to_value(&cmd.config).unwrap_or_default(), // $7
            prefix,                                                // $8
            cmd.created_by,                                        // $9
        )
        .fetch_one(&self.pool)
        .await?;

        Property::try_from(row)
    }

    async fn update(&self, cmd: UpdatePropertyCommand) -> Result<Property, AppError> {
        let row = sqlx::query_as!(
            PropertyRow,
            r#"
            UPDATE properties
            SET
                name       = COALESCE($3, name),
                address    = COALESCE($4, address),
                city       = COALESCE($5, city),
                updated_at = now()
            WHERE id = $1 AND agency_id = $2
            RETURNING
                id, agency_id, name, address, city, country_code,
                property_type,
                config AS "config: sqlx::types::Json<serde_json::Value>",
                work_order_prefix, work_order_seq,
                created_by, created_at, updated_at
            "#,
            cmd.id,
            cmd.agency_id,
            cmd.name,
            cmd.address,
            cmd.city
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("property {}", cmd.id)))?;

        Property::try_from(row)
    }

    async fn delete(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!(
            "DELETE FROM properties WHERE id = $1 AND agency_id = $2",
            id,
            agency_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("property {id}")));
        }
        Ok(())
    }

    async fn count_for_agency(&self, agency_id: Uuid) -> Result<i64, AppError> {
        let row = sqlx::query!(
            "SELECT COUNT(*) AS count FROM properties WHERE agency_id = $1",
            agency_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.count.unwrap_or(0))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn property_type_str(pt: &PropertyType) -> &'static str {
    match pt {
        PropertyType::Residential => "residential",
        PropertyType::Commercial => "commercial",
        PropertyType::Community => "community",
        PropertyType::Student => "student",
        PropertyType::Affordable => "affordable",
    }
}
