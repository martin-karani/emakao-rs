use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::property_repository::{PropertyFilter, PropertyRepository},
    },
    domain::{
        enums::PropertyType,
        property::{CreatePropertyCommand, Property, PropertyConfig, UpdatePropertyCommand},
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

#[async_trait]
impl PropertyRepository for PgPropertyRepo {
    async fn find_all(&self, filter: PropertyFilter) -> Result<Vec<Property>, AppError> {
        let rows = sqlx::query_as!(
            PropertyRow,
            r#"
            SELECT
                id, agency_id, name, address, city, country_code,
                property_type, config AS "config: sqlx::types::Json<serde_json::Value>",
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
                property_type, config AS "config: sqlx::types::Json<serde_json::Value>",
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
        let property_type_str = serde_json::to_value(&cmd.property_type)
            .map_err(|e| AppError::ExternalService(e.to_string()))?
            .as_str()
            .unwrap_or_default()
            .to_owned();

        let config_json = serde_json::to_value(&cmd.config)
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        let row = sqlx::query_as!(
            PropertyRow,
            r#"
            INSERT INTO properties (
                id, agency_id, name, address, city,
                property_type, config, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, agency_id, name, address, city, country_code,
                property_type, config AS "config: sqlx::types::Json<serde_json::Value>",
                created_by, created_at, updated_at
            "#,
            Uuid::new_v4(),
            cmd.agency_id,
            cmd.name,
            cmd.address,
            cmd.city,
            property_type_str,
            config_json,
            cmd.created_by
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
                property_type, config AS "config: sqlx::types::Json<serde_json::Value>",
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
