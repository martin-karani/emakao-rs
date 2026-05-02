use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::auth_repository::{AuthRepository, CreateUserCommand},
    },
    domain::auth::StoredUser,
};

pub struct PgAuthRepo {
    pool: PgPool,
}

impl PgAuthRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    agency_id: Uuid,
    email: String,
    password_hash: String,
    role: String,
    is_active: bool,
}

impl From<UserRow> for StoredUser {
    fn from(r: UserRow) -> Self {
        Self {
            id: r.id,
            agency_id: r.agency_id,
            email: r.email,
            password_hash: r.password_hash,
            role: r.role,
            is_active: r.is_active,
        }
    }
}

#[async_trait]
impl AuthRepository for PgAuthRepo {
    async fn find_by_email(
        &self,
        agency_id: Uuid,
        email: &str,
    ) -> Result<Option<StoredUser>, AppError> {
        let row = sqlx::query_as!(
            UserRow,
            r#"
            SELECT id, agency_id, email, password_hash, role, is_active
            FROM users
            WHERE agency_id = $1 AND email = $2
            LIMIT 1
            "#,
            agency_id,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(StoredUser::from))
    }

    async fn create(&self, cmd: CreateUserCommand) -> Result<StoredUser, AppError> {
        let row = sqlx::query_as!(
            UserRow,
            r#"
            INSERT INTO users (id, agency_id, email, password_hash, role, is_active)
            VALUES ($1, $2, $3, $4, $5, true)
            RETURNING id, agency_id, email, password_hash, role, is_active
            "#,
            cmd.id,
            cmd.agency_id,
            cmd.email,
            cmd.password_hash,
            cmd.role
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(StoredUser::from(row))
    }

    async fn email_exists(&self, agency_id: Uuid, email: &str) -> Result<bool, AppError> {
        let row = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE agency_id = $1 AND email = $2) AS exists",
            agency_id,
            email
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.exists.unwrap_or(false))
    }
}
