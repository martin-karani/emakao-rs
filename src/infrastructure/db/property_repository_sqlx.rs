use async_trait::async_trait;
use sqlx::{Column, PgPool, Row};
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

// ── Row type ──────────────────────────────────────────────────────────────────

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
    unit_types: sqlx::types::Json<serde_json::Value>,
    photos: sqlx::types::Json<Vec<String>>,
    documents: sqlx::types::Json<serde_json::Value>,
    work_order_prefix: Option<String>,
    work_order_seq: Option<i32>,
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

        let unit_types = serde_json::from_value(row.unit_types.0)
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        let documents = serde_json::from_value(row.documents.0)
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
            unit_types,
            photos: row.photos.0,
            documents,
            maintenance: crate::domain::property::PropertyMaintenanceConfig {
                work_order_prefix: row.work_order_prefix.unwrap_or_default(),
                work_order_seq: row.work_order_seq.unwrap_or(0),
            },
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
        let rows = sqlx::query_as::<_, PropertyRow>(
            r#"
            SELECT
                p.id, p.agency_id, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.created_by, p.created_at, p.updated_at
            FROM properties p
            LEFT JOIN property_maintenance_configs mc ON mc.property_id = p.id
            WHERE p.agency_id = $1
              AND ($2::text IS NULL OR p.property_type::text = $2)
              AND ($5::text IS NULL OR p.name ILIKE $5 OR p.address ILIKE $5 OR p.city ILIKE $5)
            ORDER BY p.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(filter.agency_id)
        .bind(filter.property_type)
        .bind(filter.limit)
        .bind(filter.offset)
        .bind(filter.search.map(|s| format!("%{}%", s)))
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut properties = Vec::new();
        for row in rows {
            properties.push(Property::try_from(row)?);
        }
        Ok(properties)
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Property>, AppError> {
        let row = sqlx::query_as::<_, PropertyRow>(
            r#"
            SELECT
                p.id, p.agency_id, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.created_by, p.created_at, p.updated_at
            FROM properties p
            LEFT JOIN property_maintenance_configs mc ON mc.property_id = p.id
            WHERE p.id = $1 AND p.agency_id = $2
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        match row {
            Some(r) => Ok(Some(Property::try_from(r)?)),
            None => Ok(None),
        }
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

        // ── 2. Fetch existing prefixes ────────────────────────────────────────
        let taken_rows =
            sqlx::query("SELECT work_order_prefix FROM property_maintenance_configs WHERE work_order_prefix LIKE $1")
                .bind(format!("{}%", &base[..base.len().min(6)]))
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::Database)?;

        let taken: HashSet<String> = taken_rows
            .into_iter()
            .map(|r| r.get::<String, _>(0))
            .collect();

        let prefix = unique_prefix(&base, &taken);

        // ── 3. Insert ─────────────────────────────────────────────────────────
        let country_code = if cmd.country_code.trim().is_empty() {
            "KE".to_string()
        } else {
            cmd.country_code.trim().to_ascii_uppercase()
        };

        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        let property_id = Uuid::now_v7();

        // Insert property
        let res = sqlx::query(
            r#"
            INSERT INTO properties (
                id, agency_id, name, address, city, country_code,
                property_type, config, unit_types, photos, documents, created_by
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7::property_type, $8, $9, $10, $11, $12
            )
            "#,
        )
        .bind(property_id)
        .bind(cmd.agency_id)
        .bind(&cmd.name)
        .bind(&cmd.address)
        .bind(&cmd.city)
        .bind(&country_code)
        .bind(property_type_str(&cmd.property_type))
        .bind(serde_json::to_value(&cmd.config).unwrap_or_default())
        .bind(serde_json::to_value(&cmd.unit_types).unwrap_or_default())
        .bind(serde_json::to_value(&cmd.photos).unwrap_or_default())
        .bind(serde_json::to_value(&cmd.documents).unwrap_or_default())
        .bind(cmd.created_by)
        .execute(&mut *tx)
        .await;

        if let Err(e) = res {
            tracing::error!(error = %e, "PgPropertyRepo::create: INSERT properties failed");
            return Err(AppError::Database(e));
        }

        // Insert maintenance config
        sqlx::query(
            "INSERT INTO property_maintenance_configs (property_id, work_order_prefix) VALUES ($1, $2)",
        )
        .bind(property_id)
        .bind(&prefix)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        // ── 4. Link Owners ────────────────────────────────────────────────────
        for owner_id in cmd.owner_ids {
            sqlx::query(
                "INSERT INTO property_owners (property_id, owner_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(property_id)
            .bind(owner_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }

        // ── 5. Link Agents ────────────────────────────────────────────────────
        for agent_id in cmd.agent_ids {
            sqlx::query(
                "INSERT INTO property_agents (property_id, user_id, agency_id) \
                 VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(property_id)
            .bind(agent_id)
            .bind(cmd.agency_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }

        tx.commit().await.map_err(AppError::Database)?;

        // ── 6. Fetch the full row ─────────────────────────────────────────────
        let row = sqlx::query(
            r#"
            SELECT
                p.id, p.agency_id, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.created_by, p.created_at, p.updated_at
            FROM properties p
            LEFT JOIN property_maintenance_configs mc ON mc.property_id = p.id
            WHERE p.id = $1
            "#,
        )
        .bind(property_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let photos: sqlx::types::Json<Vec<String>> = row.try_get("photos").map_err(|e| {
            tracing::error!(error = %e, "PgPropertyRepo::create: failed to get 'photos' column");
            AppError::Database(e)
        })?;

        let property_row = PropertyRow {
            id: row.get("id"),
            agency_id: row.get("agency_id"),
            name: row.get("name"),
            address: row.get("address"),
            city: row.get("city"),
            country_code: row.get("country_code"),
            property_type: row.get("property_type"),
            config: row.get("config"),
            unit_types: row.get("unit_types"),
            photos,
            documents: row.get("documents"),
            work_order_prefix: row.get("work_order_prefix"),
            work_order_seq: row.get("work_order_seq"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Property::try_from(property_row)
    }

    async fn update(&self, cmd: UpdatePropertyCommand) -> Result<Property, AppError> {
        // Update properties table
        let updated = sqlx::query(
            r#"
            UPDATE properties
            SET
                name       = COALESCE($3, name),
                address    = COALESCE($4, address),
                city       = COALESCE($5, city),
                updated_at = now()
            WHERE id = $1 AND agency_id = $2
            "#,
        )
        .bind(cmd.id)
        .bind(cmd.agency_id)
        .bind(cmd.name)
        .bind(cmd.address)
        .bind(cmd.city)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        if updated.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("property {}", cmd.id)));
        }

        // Update maintenance config
        sqlx::query(
            r#"
            UPDATE property_maintenance_configs
            SET
                work_order_prefix = COALESCE($2, work_order_prefix),
                updated_at        = now()
            WHERE property_id = $1
            "#,
        )
        .bind(cmd.id)
        .bind(cmd.work_order_prefix)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        // Fetch the updated row
        let row = sqlx::query_as::<_, PropertyRow>(
            r#"
            SELECT
                p.id, p.agency_id, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.created_by, p.created_at, p.updated_at
            FROM properties p
            LEFT JOIN property_maintenance_configs mc ON mc.property_id = p.id
            WHERE p.id = $1
            "#,
        )
        .bind(cmd.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("property {}", cmd.id)))?;

        Property::try_from(row)
    }

    async fn delete(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM properties WHERE id = $1 AND agency_id = $2")
            .bind(id)
            .bind(agency_id)
            .execute(&self.pool)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("property {id}")));
        }
        Ok(())
    }

    async fn count_for_agency(&self, agency_id: Uuid) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) FROM properties WHERE agency_id = $1")
            .bind(agency_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.get::<i64, _>(0))
    }
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn property_type_str(pt: &PropertyType) -> &'static str {
    match pt {
        PropertyType::Residential => "residential",
        PropertyType::Multifamily => "multifamily",
        PropertyType::Commercial => "commercial",
        PropertyType::Community => "community",
        PropertyType::Student => "student",
        PropertyType::AffordableHousing | PropertyType::Affordable => "affordable_housing",
    }
}
