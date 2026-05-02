use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::resident_repository::{CreateResidentCommand, ResidentRepository},
    },
    domain::resident::{PortalStatus, Resident},
};

pub struct PgResidentRepo {
    pool: PgPool,
}

impl PgResidentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgResidentRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ResidentRow {
    id: Uuid,
    user_id: Uuid,
    first_name: String,
    last_name: String,
    email: String,
    phone: Option<String>,
    national_id: Option<String>,
    portal_status: String,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn parse_portal_status(s: &str) -> PortalStatus {
    match s {
        "active" => PortalStatus::Active,
        "suspended" => PortalStatus::Suspended,
        _ => PortalStatus::Invited,
    }
}

impl From<ResidentRow> for Resident {
    fn from(r: ResidentRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            first_name: r.first_name,
            last_name: r.last_name,
            email: r.email,
            phone: r.phone,
            national_id: r.national_id,
            portal_status: parse_portal_status(&r.portal_status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl ResidentRepository for PgResidentRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Resident>, AppError> {
        let rows = sqlx::query_as!(
            ResidentRow,
            r#"
            SELECT r.id, r.user_id, r.first_name, r.last_name, r.email,
                   r.phone, r.national_id, r.portal_status, r.created_at, r.updated_at
            FROM residents r
            JOIN users u ON u.id = r.user_id
            WHERE u.agency_id = $1
            ORDER BY r.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            agency_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Resident::from).collect())
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Resident>, AppError> {
        let row = sqlx::query_as!(
            ResidentRow,
            r#"
            SELECT r.id, r.user_id, r.first_name, r.last_name, r.email,
                   r.phone, r.national_id, r.portal_status, r.created_at, r.updated_at
            FROM residents r
            JOIN users u ON u.id = r.user_id
            WHERE r.id = $1 AND u.agency_id = $2
            "#,
            id,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Resident::from))
    }

    async fn find_by_email(
        &self,
        agency_id: Uuid,
        email: &str,
    ) -> Result<Option<Resident>, AppError> {
        let row = sqlx::query_as!(
            ResidentRow,
            r#"
            SELECT r.id, r.user_id, r.first_name, r.last_name, r.email,
                   r.phone, r.national_id, r.portal_status, r.created_at, r.updated_at
            FROM residents r
            JOIN users u ON u.id = r.user_id
            WHERE r.email = $1 AND u.agency_id = $2
            LIMIT 1
            "#,
            email,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Resident::from))
    }

    async fn create(&self, cmd: CreateResidentCommand) -> Result<Resident, AppError> {
        let mut tx = self.pool.begin().await?;

        let user_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO users (id, agency_id, email, password_hash, role, is_active)
            VALUES ($1, $2, $3, '', 'resident', false)
            ON CONFLICT (agency_id, email) DO NOTHING
            "#,
            user_id,
            cmd.agency_id,
            cmd.email
        )
        .execute(&mut *tx)
        .await?;

        let real_user_id: Uuid = sqlx::query_scalar!(
            "SELECT id FROM users WHERE agency_id = $1 AND email = $2",
            cmd.agency_id,
            cmd.email
        )
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query_as!(
            ResidentRow,
            r#"
            INSERT INTO residents (
                id, user_id, first_name, last_name, email,
                phone, national_id, portal_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'invited')
            RETURNING
                id, user_id, first_name, last_name, email,
                phone, national_id, portal_status, created_at, updated_at
            "#,
            Uuid::new_v4(),
            real_user_id,
            cmd.first_name,
            cmd.last_name,
            cmd.email,
            cmd.phone,
            cmd.national_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Resident::from(row))
    }
}
