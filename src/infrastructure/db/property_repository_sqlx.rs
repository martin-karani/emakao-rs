use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use std::collections::HashSet;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::property_repository::{
            PropertyFilter, PropertyRepository, PropertySummaryQuery, PropertyTeamMember,
        },
    },
    domain::{
        dashboard::{ExpiringLease, MaintenanceSummary},
        enums::PropertyType,
        property::{CreatePropertyCommand, Property, PropertyConfig, UpdatePropertyCommand},
        property_summary::{PropertyRentSummary, PropertyStats, PropertySummary},
        work_order_code::{candidate_prefix, unique_prefix},
    },
};

#[derive(Debug, sqlx::FromRow)]
struct PropertyRow {
    id: Uuid,
    agency_id: Uuid,
    slug: String,
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
    policies: Option<sqlx::types::Json<serde_json::Value>>,
    created_by: Uuid,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
    deleted_at: Option<time::OffsetDateTime>,
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

        let policies = match row.policies {
            Some(p) => serde_json::from_value(p.0).ok(),
            None => None,
        };

        Ok(Property {
            id: row.id,
            agency_id: row.agency_id,
            slug: row.slug,
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
            policies,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
        })
    }
}

pub struct PgPropertyRepo {
    pool: PgPool,
}

impl From<PgPool> for PgPropertyRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Repository ────────────────────────────────────────────────────────────────

#[async_trait]
impl PropertyRepository for PgPropertyRepo {
    async fn get_team(
        &self,
        agency_id: Uuid,
        property_id: Uuid,
    ) -> Result<Vec<PropertyTeamMember>, AppError> {
        // Query owners
        let owner_rows = sqlx::query(
            r#"
            SELECT
                o.id,
                o.user_id,
                o.first_name || ' ' || o.last_name as name,
                o.email,
                o.phone,
                o.portal_status::text as status
            FROM owners o
            JOIN property_owners po ON po.owner_id = o.id
            JOIN properties p ON p.id = po.property_id
            WHERE po.property_id = $1 AND p.agency_id = $2
            "#,
        )
        .bind(property_id)
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        // Query caretakers
        let caretaker_rows = sqlx::query(
            r#"
            SELECT
                c.id,
                c.user_id,
                c.first_name || ' ' || c.last_name as name,
                c.email,
                c.phone,
                c.is_active
            FROM caretakers c
            JOIN properties p ON p.id = c.property_id
            WHERE c.property_id = $1 AND p.agency_id = $2
            "#,
        )
        .bind(property_id)
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let mut team = Vec::new();

        for row in owner_rows {
            team.push(PropertyTeamMember {
                id: row.get("id"),
                user_id: row.try_get("user_id").ok(),
                name: row.get("name"),
                email: row.try_get("email").ok(),
                phone: row.try_get("phone").ok(),
                role: "owner".to_string(),
                status: row.get::<String, _>("status"),
            });
        }

        for row in caretaker_rows {
            let is_active: bool = row.get("is_active");
            let user_id: Option<Uuid> = row.try_get("user_id").ok();

            // If user_id is missing, they haven't been linked to a platform user yet, meaning they haven't accepted the invite.
            let status = if user_id.is_none() {
                "invited".to_string()
            } else if is_active {
                "active".to_string()
            } else {
                "inactive".to_string()
            };

            team.push(PropertyTeamMember {
                id: row.get("id"),
                user_id,
                name: row.get("name"),
                email: row.try_get("email").ok(),
                phone: row.try_get("phone").ok(),
                role: "caretaker".to_string(),
                status,
            });
        }

        Ok(team)
    }

    async fn get_summary(&self, query: PropertySummaryQuery) -> Result<PropertySummary, AppError> {
        let generated_at = OffsetDateTime::now_utc();

        // 1. Fetch property basic info (only if not deleted)
        let prop = self
            .find_by_id(query.agency_id, query.property_id)
            .await?
            .ok_or_else(|| AppError::NotFound("property".into()))?;

        // Double-check property is not deleted
        if prop.deleted_at.is_some() {
            return Err(AppError::NotFound("property".into()));
        }

        // 2. Stats
        let stats_row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM units WHERE property_id = $1)::BIGINT AS total_units,
                (SELECT COUNT(*) FROM units WHERE property_id = $1 AND status::text = 'occupied')::BIGINT AS occupied_units,
                (SELECT COUNT(*) FROM agreements WHERE property_id = $1 AND status::text = 'active')::BIGINT AS active_leases,
                (SELECT COUNT(*) FROM work_orders WHERE property_id = $1 AND status::text NOT IN ('completed', 'cancelled'))::BIGINT AS open_work_orders
            "#
        )
        .bind(query.property_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let total_units: i64 = stats_row.get("total_units");
        let occupied_units: i64 = stats_row.get("occupied_units");
        let active_leases: i64 = stats_row.get("active_leases");
        let open_work_orders: i64 = stats_row.get("open_work_orders");

        let occupancy_rate = if total_units > 0 {
            Decimal::from(occupied_units * 100) / Decimal::from(total_units)
        } else {
            Decimal::ZERO
        };

        let stats = PropertyStats {
            total_units,
            occupied_units,
            vacant_units: total_units - occupied_units,
            occupancy_rate_pct: occupancy_rate,
            active_leases,
            open_work_orders,
        };

        // 3. Expiring leases
        let lease_rows = sqlx::query(
            r#"
            SELECT
                a.id AS agreement_id,
                u.id AS unit_id,
                u.unit_number,
                r.id AS resident_id,
                r.first_name || ' ' || r.last_name AS resident_name,
                a.end_date,
                (a.end_date - CURRENT_DATE)::INTEGER AS days_until_expiry,
                a.rent_amount_kes
            FROM agreements a
            JOIN units u ON u.id = a.unit_id
            JOIN residents r ON r.id = a.resident_id
            WHERE a.property_id = $1
              AND a.status::text = 'active'
              AND a.end_date IS NOT NULL
              AND a.end_date <= CURRENT_DATE + ($2 * INTERVAL '1 day')
            ORDER BY a.end_date ASC
            LIMIT 10
            "#,
        )
        .bind(query.property_id)
        .bind(query.expiring_lease_days as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let expiring_leases = lease_rows
            .into_iter()
            .map(|r| ExpiringLease {
                agreement_id: r.get("agreement_id"),
                unit_id: r.get("unit_id"),
                unit_number: r.get("unit_number"),
                property_id: query.property_id,
                property_name: prop.name.clone(),
                resident_id: r.get("resident_id"),
                resident_name: r.get("resident_name"),
                end_date: r.get::<Option<Date>, _>("end_date").unwrap_or_else(|| {
                    Date::from_calendar_date(2000, time::Month::January, 1).unwrap()
                }),
                days_until_expiry: r.get::<Option<i32>, _>("days_until_expiry").unwrap_or(0) as i64,
                rent_amount_kes: r.get("rent_amount_kes"),
            })
            .collect();

        // 4. Pending maintenance
        let maint_rows = sqlx::query(
            r#"
            SELECT
                wo.id AS work_order_id,
                wo.code,
                wo.title,
                wo.priority::text AS priority,
                wo.status::text AS status,
                u.id AS unit_id,
                u.unit_number,
                wo.created_at,
                (EXTRACT(EPOCH FROM (now() - wo.created_at)) / 86400)::BIGINT AS days_open
            FROM work_orders wo
            LEFT JOIN units u ON u.id = wo.unit_id
            WHERE wo.property_id = $1
              AND wo.status::text NOT IN ('completed', 'cancelled')
            ORDER BY
                CASE wo.priority::text
                    WHEN 'emergency' THEN 0
                    WHEN 'high' THEN 1
                    WHEN 'medium' THEN 2
                    ELSE 3
                END,
                wo.created_at ASC
            LIMIT 10
            "#,
        )
        .bind(query.property_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let pending_maintenance = maint_rows
            .into_iter()
            .map(|r| MaintenanceSummary {
                work_order_id: r.get("work_order_id"),
                code: r.get("code"),
                title: r.get("title"),
                priority: r.get::<Option<String>, _>("priority").unwrap_or_default(),
                status: r.get::<Option<String>, _>("status").unwrap_or_default(),
                property_id: query.property_id,
                property_name: prop.name.clone(),
                unit_id: r.get("unit_id"),
                unit_number: r.get("unit_number"),
                created_at: r.get("created_at"),
                days_open: r.get::<Option<i64>, _>("days_open").unwrap_or(0),
            })
            .collect();

        // 5. Rent collection (current month)
        let rent_row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type::text IN ('rent', 'utility', 'late_fee')), 0) AS expected,
                COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type::text IN ('payment_mpesa', 'payment_bank', 'payment_cash')), 0) AS collected
            FROM ledger_entries le
            JOIN agreements a ON a.id = le.agreement_id
            WHERE a.property_id = $1
              AND le.posted_at >= date_trunc('month', now())
            "#
        )
        .bind(query.property_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let expected: Decimal = rent_row.get("expected");
        let collected: Decimal = rent_row.get("collected");
        let outstanding = expected - collected;
        let collection_rate = if expected > Decimal::ZERO {
            (collected / expected) * Decimal::from(100)
        } else {
            Decimal::from(100)
        };

        let rent_collection = PropertyRentSummary {
            total_expected_kes: expected,
            total_collected_kes: collected,
            outstanding_kes: outstanding,
            collection_rate_pct: collection_rate,
        };

        Ok(PropertySummary {
            generated_at,
            property_id: query.property_id,
            name: prop.name,
            slug: prop.slug,
            stats,
            expiring_leases,
            pending_maintenance,
            rent_collection,
        })
    }

    async fn find_all(&self, filter: PropertyFilter) -> Result<Vec<Property>, AppError> {
        let rows = sqlx::query_as::<_, PropertyRow>(
            r#"
            SELECT
                p.id, p.agency_id, p.slug, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.policies, p.created_by, p.created_at, p.updated_at, p.deleted_at
            FROM properties p
            LEFT JOIN property_maintenance_configs mc ON mc.property_id = p.id
            WHERE p.agency_id = $1
              AND p.deleted_at IS NULL
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
                p.id, p.agency_id, p.slug, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.policies, p.created_by, p.created_at, p.updated_at, p.deleted_at
            FROM properties p
            LEFT JOIN property_maintenance_configs mc ON mc.property_id = p.id
            WHERE p.id = $1 AND p.agency_id = $2 AND p.deleted_at IS NULL
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

    async fn find_by_slug(
        &self,
        agency_id: Uuid,
        slug: &str,
    ) -> Result<Option<Property>, AppError> {
        let row = sqlx::query_as::<_, PropertyRow>(
            r#"
            SELECT
                p.id, p.agency_id, p.slug, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.policies, p.created_by, p.created_at, p.updated_at, p.deleted_at
            FROM properties p
            LEFT JOIN property_maintenance_configs mc ON mc.property_id = p.id
            WHERE p.agency_id = $1 AND p.slug = $2 AND p.deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(agency_id)
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        match row {
            Some(r) => Ok(Some(Property::try_from(r)?)),
            None => Ok(None),
        }
    }

    async fn find_by_name(
        &self,
        agency_id: Uuid,
        name: &str,
    ) -> Result<Option<Property>, AppError> {
        let row = sqlx::query_as::<_, PropertyRow>(
            r#"
            SELECT
                p.id, p.agency_id, p.slug, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.policies, p.created_by, p.created_at, p.updated_at, p.deleted_at
            FROM properties p
            LEFT JOIN property_maintenance_configs mc ON mc.property_id = p.id
            WHERE p.agency_id = $1 AND p.name ILIKE $2 AND p.deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(agency_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        match row {
            Some(r) => Ok(Some(Property::try_from(r)?)),
            None => Ok(None),
        }
    }

    async fn list_slugs_for_agency(&self, agency_id: Uuid) -> Result<Vec<String>, AppError> {
        // Only list slugs for non-deleted properties
        let rows =
            sqlx::query("SELECT slug FROM properties WHERE agency_id = $1 AND deleted_at IS NULL")
                .bind(agency_id)
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .map(|row| row.get::<String, _>("slug"))
            .collect())
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
        let policies_val = cmd
            .policies
            .map(|p| serde_json::to_value(p).unwrap_or_default());
        let res = sqlx::query(
            r#"
            INSERT INTO properties (
                id, agency_id, slug, name, address, city, country_code,
                property_type, config, unit_types, photos, documents, created_by,
                policies
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8::property_type, $9, $10, $11, $12, $13, $14
            )
            "#,
        )
        .bind(property_id)
        .bind(cmd.agency_id)
        .bind(&cmd.slug)
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
        .bind(policies_val)
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
                p.id, p.agency_id, p.slug, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.policies, p.created_by, p.created_at, p.updated_at
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
            slug: row.get("slug"),
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
            policies: row.get("policies"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            deleted_at: row.get("deleted_at"),
        };

        Property::try_from(property_row)
    }

    async fn update(&self, cmd: UpdatePropertyCommand) -> Result<Property, AppError> {
        // Update properties table
        let policies_val = cmd
            .policies
            .map(|p| serde_json::to_value(p).unwrap_or_default());
        let updated = sqlx::query(
            r#"
            UPDATE properties
            SET
                name       = COALESCE($3, name),
                address    = COALESCE($4, address),
                city       = COALESCE($5, city),
                policies   = COALESCE($6, policies),
                slug       = COALESCE($7, slug),
                updated_at = now()
            WHERE id = $1 AND agency_id = $2
            "#,
        )
        .bind(cmd.id)
        .bind(cmd.agency_id)
        .bind(cmd.name)
        .bind(cmd.address)
        .bind(cmd.city)
        .bind(policies_val)
        .bind(cmd.slug)
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
                p.id, p.agency_id, p.slug, p.name, p.address, p.city, p.country_code,
                p.property_type::text as property_type, p.config, p.unit_types, p.photos, p.documents,
                mc.work_order_prefix, mc.work_order_seq,
                p.policies, p.created_by, p.created_at, p.updated_at
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
        // Soft delete: set deleted_at timestamp instead of hard delete
        let result = sqlx::query(
            "UPDATE properties SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND agency_id = $2 AND deleted_at IS NULL"
        )
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
        // Count only non-deleted properties
        let row = sqlx::query(
            "SELECT COUNT(*) FROM properties WHERE agency_id = $1 AND deleted_at IS NULL",
        )
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
