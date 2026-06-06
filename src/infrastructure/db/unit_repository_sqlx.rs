// src/infrastructure/db/unit_repository_sqlx.rs
//
// sqlx implementation of the UnitRepository port.
// All queries target the `units` table in the agency Postgres schema.
// `create_batch` uses an explicit transaction so partial inserts are rolled back.

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::unit_repository::UnitRepository},
    domain::{
        enums::UnitStatus,
        property::{CreateUnitCommand, Unit, UpdateUnitCommand},
    },
};

pub struct PgUnitRepo {
    pool: PgPool,
}

impl PgUnitRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgUnitRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Row type ──────────────────────────────────────────────────────────────────

/// Mirrors the `units` table exactly. The `status` column is read as `TEXT`
/// then parsed, following the same pattern as `property_type` in
/// `property_repository_sqlx.rs`.
#[derive(sqlx::FromRow)]
struct UnitRow {
    id: Uuid,
    property_id: Uuid,
    unit_type_id: Option<Uuid>,
    parent_unit_id: Option<Uuid>,
    unit_number: String,
    floor: Option<i32>,
    size_sqm: Option<Decimal>,
    bedrooms: Option<i16>,
    bathrooms: Option<i16>,
    rent_amount_kes: Decimal,
    deposit_kes: Decimal,
    status: String,
    photos: Option<sqlx::types::Json<Vec<String>>>,
    description: Option<String>,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

impl TryFrom<UnitRow> for Unit {
    type Error = AppError;

    fn try_from(row: UnitRow) -> Result<Self, Self::Error> {
        let status: UnitStatus = serde_json::from_value(serde_json::Value::String(row.status))
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        Ok(Unit {
            id: row.id,
            property_id: row.property_id,
            unit_type_id: row.unit_type_id,
            parent_unit_id: row.parent_unit_id,
            unit_number: row.unit_number,
            floor: row.floor,
            size_sqm: row.size_sqm.map(|v| {
                // Convert NUMERIC → f64 via string to avoid precision loss
                v.to_string().parse::<f64>().unwrap_or(0.0)
            }),
            bedrooms: row.bedrooms,
            bathrooms: row.bathrooms,
            rent_amount_kes: row.rent_amount_kes,
            deposit_kes: row.deposit_kes,
            status,
            photos: row.photos.map(|p| p.0).unwrap_or_default(),
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

// ── SELECT fragment ───────────────────────────────────────────────────────────

const SELECT: &str = r#"
    SELECT
        id, property_id, unit_type_id, parent_unit_id, unit_number,
        floor, size_sqm, bedrooms, bathrooms,
        rent_amount_kes, deposit_kes,
        status::text AS status, photos, description,
        created_at, updated_at
    FROM units
"#;

// ── Repository impl ───────────────────────────────────────────────────────────

#[async_trait]
impl UnitRepository for PgUnitRepo {
    async fn find_all_for_property(&self, property_id: Uuid) -> Result<Vec<Unit>, AppError> {
        let rows = sqlx::query_as::<_, UnitRow>(&format!(
            "{} WHERE property_id = $1 ORDER BY unit_number",
            SELECT
        ))
        .bind(property_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        rows.into_iter().map(Unit::try_from).collect()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Unit>, AppError> {
        let row = sqlx::query_as::<_, UnitRow>(&format!("{} WHERE id = $1", SELECT))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::InternalServer(e.to_string()))?;

        row.map(Unit::try_from).transpose()
    }

    async fn property_id_for_unit(&self, unit_id: Uuid) -> Result<Option<Uuid>, AppError> {
        let row = sqlx::query("SELECT property_id FROM units WHERE id = $1")
            .bind(unit_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| r.get::<Uuid, _>(0)))
    }

    async fn create(&self, cmd: CreateUnitCommand) -> Result<Unit, AppError> {
        let row = sqlx::query_as::<_, UnitRow>(
            r#"
            INSERT INTO units (
                id, property_id, unit_type_id, unit_number, floor, size_sqm,
                bedrooms, bathrooms, rent_amount_kes, deposit_kes, photos, description
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
            )
            RETURNING
                id, property_id, unit_type_id, parent_unit_id, unit_number,
                floor, size_sqm, bedrooms, bathrooms,
                rent_amount_kes, deposit_kes,
                status::text AS status, photos, description,
                created_at, updated_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(cmd.property_id)
        .bind(cmd.unit_type_id)
        .bind(&cmd.unit_number)
        .bind(cmd.floor)
        .bind(cmd.size_sqm)
        .bind(cmd.bedrooms)
        .bind(cmd.bathrooms)
        .bind(cmd.rent_amount_kes)
        .bind(cmd.deposit_kes)
        .bind(sqlx::types::Json(Vec::<String>::new())) // photos
        .bind(cmd.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| {
            // Duplicate unit_number within the property
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                AppError::Conflict(format!(
                    "unit_number '{}' already exists in this property",
                    &cmd.unit_number
                ))
            } else {
                AppError::InternalServer(e.to_string())
            }
        })?;

        Unit::try_from(row)
    }

    async fn create_batch(&self, cmds: Vec<CreateUnitCommand>) -> Result<Vec<Unit>, AppError> {
        if cmds.is_empty() {
            return Ok(vec![]);
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let mut units = Vec::with_capacity(cmds.len());

        for cmd in cmds {
            let row = sqlx::query_as::<_, UnitRow>(
                r#"
                INSERT INTO units (
                    id, property_id, unit_type_id, unit_number, floor, size_sqm,
                    bedrooms, bathrooms, rent_amount_kes, deposit_kes, description
                )
                VALUES (uuidv7(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING
                    id, property_id, unit_type_id, parent_unit_id, unit_number,
                    floor, size_sqm, bedrooms, bathrooms,
                    rent_amount_kes, deposit_kes,
                    status::text AS status, photos, description,
                    created_at, updated_at
                "#,
            )
            .bind(cmd.property_id)
            .bind(cmd.unit_type_id)
            .bind(&cmd.unit_number)
            .bind(cmd.floor)
            .bind(cmd.size_sqm)
            .bind(cmd.bedrooms)
            .bind(cmd.bathrooms)
            .bind(cmd.rent_amount_kes)
            .bind(cmd.deposit_kes)
            .bind(cmd.description)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| {
                if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                    AppError::Conflict(format!(
                        "unit_number '{}' already exists in this property",
                        &cmd.unit_number
                    ))
                } else {
                    AppError::InternalServer(e.to_string())
                }
            })?;

            units.push(Unit::try_from(row)?);
        }

        tx.commit()
            .await
            .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(units)
    }

    async fn update(&self, cmd: UpdateUnitCommand) -> Result<Unit, AppError> {
        // Build a dynamic UPDATE statement — only include columns that have
        // `Some` values so we don't accidentally nullify unset fields.
        let row = sqlx::query_as::<_, UnitRow>(
            r#"
            UPDATE units
            SET
                unit_number     = COALESCE($2, unit_number),
                floor           = CASE WHEN $3 THEN $4   ELSE floor       END,
                size_sqm        = CASE WHEN $5 THEN $6   ELSE size_sqm    END,
                bedrooms        = CASE WHEN $7 THEN $8   ELSE bedrooms    END,
                bathrooms       = CASE WHEN $9 THEN $10  ELSE bathrooms   END,
                rent_amount_kes = COALESCE($11, rent_amount_kes),
                deposit_kes     = COALESCE($12, deposit_kes),
                status          = COALESCE($13::unit_status, status),
                description     = CASE WHEN $14 THEN $15 ELSE description END,
                unit_type_id    = CASE WHEN $16 THEN $17 ELSE unit_type_id END,
                updated_at      = now()
            WHERE id = $1
            RETURNING
                id, property_id, unit_type_id, parent_unit_id, unit_number,
                floor, size_sqm, bedrooms, bathrooms,
                rent_amount_kes, deposit_kes,
                status::text AS status, photos, description,
                created_at, updated_at
            "#,
        )
        .bind(cmd.id)
        // unit_number — None keeps old value via COALESCE
        .bind(cmd.unit_number)
        // floor — CASE pattern: ($3=true means caller wants to set, $4 is new value)
        .bind(cmd.floor.is_some())
        .bind(cmd.floor.flatten())
        // size_sqm
        .bind(cmd.size_sqm.is_some())
        .bind(cmd.size_sqm.flatten())
        // bedrooms
        .bind(cmd.bedrooms.is_some())
        .bind(cmd.bedrooms.flatten())
        // bathrooms
        .bind(cmd.bathrooms.is_some())
        .bind(cmd.bathrooms.flatten())
        // rent_amount_kes
        .bind(cmd.rent_amount_kes)
        // deposit_kes
        .bind(cmd.deposit_kes)
        // status — cast to enum type
        .bind(cmd.status.map(|s| match s {
            UnitStatus::Vacant => "vacant",
            UnitStatus::Occupied => "occupied",
            UnitStatus::Maintenance => "maintenance",
            UnitStatus::Reserved => "reserved",
            UnitStatus::Inactive => "inactive",
        }))
        // description
        .bind(cmd.description.is_some())
        .bind(cmd.description.flatten())
        // unit_type_id
        .bind(cmd.unit_type_id.is_some())
        .bind(cmd.unit_type_id.flatten())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("unit {}", cmd.id)))?;

        Unit::try_from(row)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM units WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::InternalServer(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("unit {id}")));
        }
        Ok(())
    }

    async fn has_active_agreement(&self, unit_id: Uuid) -> Result<bool, AppError> {
        let row = sqlx::query(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM agreements
                WHERE unit_id = $1 AND status = 'active'
            )
            "#,
        )
        .bind(unit_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.get::<bool, _>(0))
    }
}
