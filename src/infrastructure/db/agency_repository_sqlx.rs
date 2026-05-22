use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::agency_repository::{AgencyRepository, CreateAgencyCommand},
    },
    domain::agency::Agency,
};

pub struct PgAgencyRepo {
    /// Always the platform pool (emakao_platform, public schema).
    pool: PgPool,
}

impl PgAgencyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AgencyRepository for PgAgencyRepo {
    async fn create(&self, cmd: CreateAgencyCommand) -> Result<Agency, AppError> {
        let row = sqlx::query(
            r#"
            INSERT INTO agencies (name, slug, schema_name, country_code, currency_code)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, slug, schema_name, fga_store_id, status::text
            "#,
        )
        .bind(&cmd.name)
        .bind(&cmd.slug)
        .bind(&cmd.schema_name)
        .bind(&cmd.country_code)
        .bind(&cmd.currency_code)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(Agency {
            id: row.get("id"),
            name: row.get("name"),
            slug: row.get("slug"),
            schema_name: row.get("schema_name"),
            fga_store_id: row.get("fga_store_id"),
            status: row.get("status"),
            country_code: cmd.country_code,
            currency_code: cmd.currency_code,
        })
    }

    async fn save_fga_store_id(&self, agency_id: Uuid, store_id: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE agencies SET fga_store_id = $1 WHERE id = $2")
            .bind(store_id)
            .bind(agency_id)
            .execute(&self.pool)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Agency>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id, status::text, country_code, currency_code
            FROM   agencies
            WHERE  slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| Agency {
            id: r.get("id"),
            name: r.get("name"),
            slug: r.get("slug"),
            schema_name: r.get("schema_name"),
            fga_store_id: r.get("fga_store_id"),
            status: r.get("status"),
            country_code: r.get("country_code"),
            currency_code: r.get("currency_code"),
        }))
    }
}
