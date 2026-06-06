use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::role_repository::{CreateRoleCommand, RoleRepository, UpdateRoleCommand},
    },
    domain::role::CustomRole,
};

pub struct PgRoleRepo {
    pool: PgPool,
}

impl PgRoleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoleRepository for PgRoleRepo {
    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<CustomRole>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, agency_id, name, permissions, is_system, created_at
            FROM custom_roles
            WHERE id = $1 AND agency_id = $2
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(map_row_to_role))
    }

    async fn find_by_name(
        &self,
        agency_id: Uuid,
        name: &str,
    ) -> Result<Option<CustomRole>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, agency_id, name, permissions, is_system, created_at
            FROM custom_roles
            WHERE agency_id = $1 AND name = $2
            "#,
        )
        .bind(agency_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(map_row_to_role))
    }

    async fn list_for_agency(&self, agency_id: Uuid) -> Result<Vec<CustomRole>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT id, agency_id, name, permissions, is_system, created_at
            FROM custom_roles
            WHERE agency_id = $1
            ORDER BY is_system DESC, name ASC
            "#,
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(map_row_to_role).collect())
    }

    async fn create(&self, cmd: CreateRoleCommand) -> Result<CustomRole, AppError> {
        let perms = serde_json::to_value(&cmd.permissions)
            .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let role_name = cmd.name.clone();
        let row = sqlx::query(
            r#"
            INSERT INTO custom_roles (id, agency_id, name, permissions, is_system)
            VALUES ($1, $2, $3, $4, false)
            RETURNING id, agency_id, name, permissions, is_system, created_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(cmd.agency_id)
        .bind(&cmd.name)
        .bind(perms)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return AppError::Conflict(format!(
                        "Role with name '{}' already exists",
                        role_name
                    ));
                }
            }
            AppError::InternalServer(e.to_string())
        })?;

        Ok(map_row_to_role(row))
    }

    async fn update(&self, cmd: UpdateRoleCommand) -> Result<CustomRole, AppError> {
        let perms = serde_json::to_value(&cmd.permissions)
            .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let row = sqlx::query(
            r#"
            UPDATE custom_roles
            SET permissions = $3
            WHERE id = $1 AND agency_id = $2 AND is_system = false
            RETURNING id, agency_id, name, permissions, is_system, created_at
            "#,
        )
        .bind(cmd.id)
        .bind(cmd.agency_id)
        .bind(perms)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Role not found or is a system role".into()))?;

        Ok(map_row_to_role(row))
    }

    async fn delete(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM custom_roles
            WHERE id = $1 AND agency_id = $2 AND is_system = false
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Role not found or is a system role".into(),
            ));
        }
        Ok(())
    }

    async fn seed_system_roles(&self, agency_id: Uuid) -> Result<(), AppError> {
        let roles = CustomRole::system_roles(agency_id);

        for role in roles {
            let perms = serde_json::to_value(&role.permissions)
                .map_err(|e| AppError::InternalServer(e.to_string()))?;

            sqlx::query(
                r#"
                INSERT INTO custom_roles (id, agency_id, name, permissions, is_system)
                VALUES ($1, $2, $3, $4, true)
                ON CONFLICT (agency_id, name) DO UPDATE SET
                    permissions = EXCLUDED.permissions,
                    is_system = true
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(agency_id)
            .bind(role.name)
            .bind(perms)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::InternalServer(e.to_string()))?;
        }

        Ok(())
    }
}

fn map_row_to_role(row: sqlx::postgres::PgRow) -> CustomRole {
    let perms_json: serde_json::Value = row.get("permissions");
    let permissions: HashSet<String> = serde_json::from_value(perms_json).unwrap_or_default();

    CustomRole {
        id: row.get("id"),
        agency_id: row.get("agency_id"),
        name: row.get("name"),
        permissions,
        is_system: row.get("is_system"),
        created_at: row.get("created_at"),
    }
}
