// src/infrastructure/db/agency_repository_sqlx.rs

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
        let row = sqlx::query!(
            r#"
            INSERT INTO agencies (name, slug, schema_name, country_code, currency_code)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, slug, schema_name, fga_store_id, status
            "#,
            cmd.name,
            cmd.slug,
            cmd.schema_name,
            cmd.country_code,
            cmd.currency_code,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Agency {
            id: row.id,
            name: row.name,
            slug: row.slug,
            schema_name: row.schema_name,
            fga_store_id: row.fga_store_id,
            status: row.status,
        })
    }

    async fn save_fga_store_id(&self, agency_id: Uuid, store_id: &str) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE agencies SET fga_store_id = $1 WHERE id = $2",
            store_id,
            agency_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Agency>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id, status
            FROM   agencies
            WHERE  slug = $1
            "#,
            slug,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Agency {
            id: r.id,
            name: r.name,
            slug: r.slug,
            schema_name: r.schema_name,
            fga_store_id: r.fga_store_id,
            status: r.status,
        }))
    }
}
