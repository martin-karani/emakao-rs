use crate::{
    application::{
        errors::AppError,
        ports::auth_repository::{
            AuthRepository, ConsumedInvite, CreateInviteTokenCommand, CreateMembershipCommand,
            CreateUserCommand, PortalIdentity, SlimAgency, UpsertPortalIndexCommand,
        },
    },
    domain::auth::StoredUser,
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgAuthRepo {
    pool: PgPool,
}

impl PgAuthRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuthRepository for PgAuthRepo {
    async fn find_staff_by_contact(
        &self,
        agency_id: Uuid,
        contact: &str,
        contact_type: &str,
    ) -> Result<Option<StoredUser>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT u.id, u.email, u.phone, u.password_hash, u.is_active,
                   u.must_change_password, uar.role, uar.agency_id
            FROM users u
            JOIN user_agency_roles uar ON uar.user_id = u.id
            WHERE uar.agency_id = $1
              AND uar.role IN ('admin','manager','agent','platform_admin')
              AND uar.is_active = true
              AND (
                  ($2 = 'email' AND u.email = $3) OR
                  ($2 = 'phone' AND u.phone = $3)
              )
            LIMIT 1
            "#,
            agency_id,
            contact_type,
            contact
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| StoredUser {
            id: r.id,
            email: r.email,
            phone: r.phone,
            password_hash: r.password_hash,
            is_active: r.is_active,
            must_change_password: r.must_change_password,
            agency_id: Some(r.agency_id),
            role: Some(r.role),
        }))
    }

    async fn find_portal_identity(
        &self,
        contact: &str,
        contact_type: &str,
        portal: &str,
    ) -> Result<Option<PortalIdentity>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT user_id, agency_id, membership_id
            FROM portal_user_index
            WHERE contact = $1 AND contact_type = $2 AND portal = $3
            "#,
            contact,
            contact_type,
            portal
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| PortalIdentity {
            user_id: r.user_id,
            agency_id: r.agency_id,
            membership_id: r.membership_id,
        }))
    }

    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<StoredUser>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT id, email, phone, password_hash, is_active, must_change_password
            FROM users WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| StoredUser {
            id: r.id,
            email: r.email,
            phone: r.phone,
            password_hash: r.password_hash,
            is_active: r.is_active,
            must_change_password: r.must_change_password,
            agency_id: None,
            role: None,
        }))
    }

    async fn find_membership(
        &self,
        user_id: Uuid,
        agency_id: Uuid,
    ) -> Result<Option<(String, bool)>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT role, is_active FROM user_agency_roles
            WHERE user_id = $1 AND agency_id = $2
            LIMIT 1
            "#,
            user_id,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| (r.role, r.is_active)))
    }

    async fn find_agency_by_slug(&self, slug: &str) -> Result<Option<SlimAgency>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id
            FROM agencies WHERE slug = $1 AND status = 'active'
            "#,
            slug
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| SlimAgency {
            id: r.id,
            name: r.name,
            slug: r.slug,
            schema_name: r.schema_name,
            fga_store_id: r.fga_store_id,
        }))
    }

    async fn find_agency_by_id(&self, id: Uuid) -> Result<Option<SlimAgency>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id
            FROM agencies WHERE id = $1 AND status = 'active'
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| SlimAgency {
            id: r.id,
            name: r.name,
            slug: r.slug,
            schema_name: r.schema_name,
            fga_store_id: r.fga_store_id,
        }))
    }

    // ── Existence guard ───────────────────────────────────────────────────────
    async fn contact_exists_for_agency(
        &self,
        agency_id: Uuid,
        contact: &str,
        contact_type: &str,
        role: &str,
    ) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM portal_user_index pui
                JOIN user_agency_roles uar ON uar.id = pui.membership_id
                WHERE pui.contact = $1
                  AND pui.contact_type = $2
                  AND uar.agency_id = $3
                  AND uar.role = $4
            )
            "#,
            contact,
            contact_type,
            agency_id,
            role
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(exists.unwrap_or(false))
    }

    // ── Write methods ─────────────────────────────────────────────────────────
    async fn create_user(&self, cmd: CreateUserCommand) -> Result<StoredUser, AppError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO users (id, email, phone, password_hash, is_active, must_change_password)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, phone, password_hash, is_active, must_change_password
            "#,
            cmd.id,
            cmd.email,
            cmd.phone,
            cmd.password_hash,
            cmd.is_active,
            cmd.must_change_password
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(StoredUser {
            id: row.id,
            email: row.email,
            phone: row.phone,
            password_hash: row.password_hash,
            is_active: row.is_active,
            must_change_password: row.must_change_password,
            agency_id: None,
            role: None,
        })
    }

    async fn create_membership(&self, cmd: CreateMembershipCommand) -> Result<Uuid, AppError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO user_agency_roles (user_id, agency_id, role)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            cmd.user_id,
            cmd.agency_id,
            cmd.role
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.id)
    }

    async fn activate_user(&self, user_id: Uuid, password_hash: &str) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE users
            SET password_hash = $2, is_active = true, must_change_password = false, updated_at = now()
            WHERE id = $1
            "#,
            user_id, password_hash
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_last_login(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE users SET last_login_at = now() WHERE id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_portal_index(&self, cmd: UpsertPortalIndexCommand) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO portal_user_index (contact, contact_type, portal, agency_id, user_id, membership_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (contact, contact_type, portal) DO UPDATE
            SET agency_id = EXCLUDED.agency_id, user_id = EXCLUDED.user_id, membership_id = EXCLUDED.membership_id
            "#,
            cmd.contact, cmd.contact_type, cmd.portal, cmd.agency_id, cmd.user_id, cmd.membership_id
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_portal_index(
        &self,
        contact: &str,
        contact_type: &str,
        portal: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM portal_user_index WHERE contact=$1 AND contact_type=$2 AND portal=$3",
            contact,
            contact_type,
            portal
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_invite_token(&self, cmd: CreateInviteTokenCommand) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO invite_tokens
                (token, user_id, agency_id, role, portal, contact, contact_type, temp_password, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            cmd.token, cmd.user_id, cmd.agency_id, cmd.role, cmd.portal,
            cmd.contact, cmd.contact_type, cmd.temp_password, cmd.expires_at
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn consume_invite_token(&self, token: &str) -> Result<ConsumedInvite, AppError> {
        let row = sqlx::query!(
            r#"
            DELETE FROM invite_tokens
            WHERE token = $1 AND expires_at > now()
            RETURNING user_id, agency_id, role, portal, contact, contact_type
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Invalid or expired invite token".into()))?;
        Ok(ConsumedInvite {
            user_id: row.user_id,
            agency_id: row.agency_id,
            role: row.role,
            portal: row.portal,
            contact: row.contact,
            contact_type: row.contact_type,
        })
    }
}
