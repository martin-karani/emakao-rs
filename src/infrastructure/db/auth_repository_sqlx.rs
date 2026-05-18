use crate::{
    application::{
        errors::AppError,
        ports::auth_repository::{
            AuthRepository, ConsumedInvite, CreateInviteTokenCommand, CreateMembershipCommand,
            CreateUserCommand, PasswordResetToken, PortalIdentity, SlimAgency, StaffMember,
            UpsertPortalIndexCommand,
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
        let row = sqlx::query(
            r#"
            SELECT u.id, u.email, u.phone, u.password_hash, u.is_active,
                   u.must_change_password, uar.role::text, uar.agency_id
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
        )
        .bind(agency_id)
        .bind(contact_type)
        .bind(contact)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| StoredUser {
            id: r.get("id"),
            email: r.get("email"),
            phone: r.get("phone"),
            password_hash: r.get("password_hash"),
            is_active: r.get("is_active"),
            must_change_password: r.get("must_change_password"),
            agency_id: Some(r.get("agency_id")),
            role: Some(r.get("role")),
        }))
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<StoredUser>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, email, phone, password_hash, is_active, must_change_password
            FROM users
            WHERE email = $1
            LIMIT 1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| StoredUser {
            id: r.get("id"),
            email: r.get("email"),
            phone: r.get("phone"),
            password_hash: r.get("password_hash"),
            is_active: r.get("is_active"),
            must_change_password: r.get("must_change_password"),
            agency_id: None,
            role: None,
        }))
    }

    async fn find_portal_identity(
        &self,
        contact: &str,
        contact_type: &str,
        portal: &str,
    ) -> Result<Option<PortalIdentity>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT user_id, agency_id, membership_id
            FROM portal_user_index
            WHERE contact = $1 AND contact_type = $2::contact_type AND portal = $3::portal_type
            "#,
        )
        .bind(contact)
        .bind(contact_type)
        .bind(portal)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| PortalIdentity {
            user_id: r.get("user_id"),
            agency_id: r.get("agency_id"),
            membership_id: r.get("membership_id"),
        }))
    }

    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<StoredUser>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, email, phone, password_hash, is_active, must_change_password
            FROM users WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| StoredUser {
            id: r.get("id"),
            email: r.get("email"),
            phone: r.get("phone"),
            password_hash: r.get("password_hash"),
            is_active: r.get("is_active"),
            must_change_password: r.get("must_change_password"),
            agency_id: None,
            role: None,
        }))
    }

    async fn find_membership(
        &self,
        user_id: Uuid,
        agency_id: Uuid,
    ) -> Result<Option<(String, bool)>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT role::text, is_active FROM user_agency_roles
            WHERE user_id = $1 AND agency_id = $2
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| (r.get("role"), r.get("is_active"))))
    }

    async fn find_agency_by_slug(&self, slug: &str) -> Result<Option<SlimAgency>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id
            FROM agencies WHERE slug = $1 AND status = 'active'
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| SlimAgency {
            id: r.get("id"),
            name: r.get("name"),
            slug: r.get("slug"),
            schema_name: r.get("schema_name"),
            fga_store_id: r.get("fga_store_id"),
        }))
    }

    async fn find_agency_by_id(&self, id: Uuid) -> Result<Option<SlimAgency>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id
            FROM agencies WHERE id = $1 AND status = 'active'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| SlimAgency {
            id: r.get("id"),
            name: r.get("name"),
            slug: r.get("slug"),
            schema_name: r.get("schema_name"),
            fga_store_id: r.get("fga_store_id"),
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
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM portal_user_index pui
                JOIN user_agency_roles uar ON uar.id = pui.membership_id
                WHERE pui.contact = $1
                  AND pui.contact_type = $2::contact_type
                  AND uar.agency_id = $3
                  AND uar.role = $4::user_role
            )
            "#,
        )
        .bind(contact)
        .bind(contact_type)
        .bind(agency_id)
        .bind(role)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(exists.unwrap_or(false))
    }

    // ── Write methods ─────────────────────────────────────────────────────────
    async fn create_user(&self, cmd: CreateUserCommand) -> Result<StoredUser, AppError> {
        let row = sqlx::query(
            r#"
            INSERT INTO users (id, email, phone, password_hash, is_active, must_change_password)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, phone, password_hash, is_active, must_change_password
            "#,
        )
        .bind(cmd.id)
        .bind(cmd.email)
        .bind(cmd.phone)
        .bind(cmd.password_hash)
        .bind(cmd.is_active)
        .bind(cmd.must_change_password)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(StoredUser {
            id: row.get("id"),
            email: row.get("email"),
            phone: row.get("phone"),
            password_hash: row.get("password_hash"),
            is_active: row.get("is_active"),
            must_change_password: row.get("must_change_password"),
            agency_id: None,
            role: None,
        })
    }

    async fn create_membership(&self, cmd: CreateMembershipCommand) -> Result<Uuid, AppError> {
        let row = sqlx::query(
            r#"
            INSERT INTO user_agency_roles (user_id, agency_id, role)
            VALUES ($1, $2, $3::user_role)
            RETURNING id
            "#,
        )
        .bind(cmd.user_id)
        .bind(cmd.agency_id)
        .bind(cmd.role)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.get("id"))
    }

    async fn activate_user(&self, user_id: Uuid, password_hash: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $2, is_active = true, must_change_password = false, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(password_hash)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn update_last_login(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn upsert_portal_index(&self, cmd: UpsertPortalIndexCommand) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO portal_user_index (contact, contact_type, portal, agency_id, user_id, membership_id)
            VALUES ($1, $2::contact_type, $3::portal_type, $4, $5, $6)
            ON CONFLICT (contact, contact_type, portal) DO UPDATE
            SET agency_id = EXCLUDED.agency_id, user_id = EXCLUDED.user_id, membership_id = EXCLUDED.membership_id
            "#,
        )
        .bind(cmd.contact)
        .bind(cmd.contact_type)
        .bind(cmd.portal)      
        .bind(cmd.agency_id)
        .bind(cmd.user_id)
        .bind(cmd.membership_id)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn remove_portal_index(
        &self,
        contact: &str,
        contact_type: &str,
        portal: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM portal_user_index WHERE contact=$1 AND contact_type=$2::contact_type AND portal=$3::portal_type",
        )
        .bind(contact)
        .bind(contact_type)
        .bind(portal)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn create_invite_token(&self, cmd: CreateInviteTokenCommand) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO invite_tokens
                (token, user_id, agency_id, role, portal, contact, contact_type, temp_password, expires_at)
            VALUES ($1, $2, $3, $4::user_role, $5::portal_type, $6, $7::contact_type, $8, $9)
            "#,
        )
        .bind(cmd.token)
        .bind(cmd.user_id)
        .bind(cmd.agency_id)
        .bind(cmd.role.as_str())     
        .bind(cmd.portal.as_str())    
        .bind(cmd.contact)
        .bind(cmd.contact_type)      
        .bind(cmd.temp_password)
        .bind(cmd.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn consume_invite_token(&self, token: &str) -> Result<ConsumedInvite, AppError> {
        let row = sqlx::query(
            r#"
            DELETE FROM invite_tokens
            WHERE token = $1 AND expires_at > now()
            RETURNING user_id, agency_id, role::text, portal::text, contact, contact_type::text
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Invalid or expired invite token".into()))?;
    
        use sqlx::Row;
        Ok(ConsumedInvite {
            user_id: row.get("user_id"),
            agency_id: row.get("agency_id"),
            role: row.get("role"),
            portal: row.get("portal"),
            contact: row.get("contact"),
            contact_type: row.get("contact_type"),
        })
    }

    // ── Password reset ────────────────────────────────────────────────────────

    async fn save_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: time::OffsetDateTime,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE
            SET token_hash = EXCLUDED.token_hash,
                expires_at = EXCLUDED.expires_at,
                created_at = now()
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn find_valid_reset_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT user_id, expires_at
            FROM password_reset_tokens
            WHERE token_hash = $1 AND expires_at > now()
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| PasswordResetToken {
            user_id: r.get("user_id"),
            expires_at: r.get("expires_at"),
        }))
    }

    async fn reset_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
        token_hash: &str,
    ) -> Result<(), AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .bind(password_hash)
            .execute(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        sqlx::query("DELETE FROM password_reset_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    // ── Staff listing / lookup ────────────────────────────────────────────────

    async fn staff_email_in_agency(&self, agency_id: Uuid, email: &str) -> Result<bool, AppError> {
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM users u
                    JOIN user_agency_roles uar ON uar.user_id = u.id
                    WHERE u.email      = $1
                      AND uar.agency_id = $2
                      AND uar.role      IN ('admin', 'manager', 'agent')
                )
                "#,
        )
        .bind(email)
        .bind(agency_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(exists.unwrap_or(false))
    }

    async fn list_staff(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StaffMember>, AppError> {
        let rows = sqlx::query(
            r#"
                SELECT
                    u.id                    AS user_id,
                    uar.id                  AS membership_id,
                    u.email,
                    uar.role::TEXT          AS role,
                    u.is_active,
                    u.must_change_password,
                    u.created_at
                FROM users u
                JOIN user_agency_roles uar ON uar.user_id = u.id
                WHERE uar.agency_id = $1
                  AND uar.role IN ('admin', 'manager', 'agent')
                ORDER BY u.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
        )
        .bind(agency_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(rows
            .into_iter()
            .map(|r| StaffMember {
                user_id: r.get("user_id"),
                membership_id: r.get("membership_id"),
                email: r.get::<Option<String>, _>("email").unwrap_or_default(),
                role: r.get("role"),
                is_active: r.get("is_active"),
                must_change_password: r.get("must_change_password"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn find_staff_member(
        &self,
        agency_id: Uuid,
        membership_id: Uuid,
    ) -> Result<Option<StaffMember>, AppError> {
        let row = sqlx::query(
            r#"
                SELECT
                    u.id                    AS user_id,
                    uar.id                  AS membership_id,
                    u.email,
                    uar.role::TEXT          AS role,
                    u.is_active,
                    u.must_change_password,
                    u.created_at
                FROM users u
                JOIN user_agency_roles uar ON uar.user_id = u.id
                WHERE uar.id        = $1
                  AND uar.agency_id = $2
                  AND uar.role IN ('admin', 'manager', 'agent')
                LIMIT 1
                "#,
        )
        .bind(membership_id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| StaffMember {
            user_id: r.get("user_id"),
            membership_id: r.get("membership_id"),
            email: r.get::<Option<String>, _>("email").unwrap_or_default(),
            role: r.get("role"),
            is_active: r.get("is_active"),
            must_change_password: r.get("must_change_password"),
            created_at: r.get("created_at"),
        }))
    }

    async fn deactivate_staff_member(
        &self,
        agency_id: Uuid,
        membership_id: Uuid,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
                UPDATE user_agency_roles
                SET    is_active = false
                WHERE  id        = $1
                  AND  agency_id = $2
                  AND  role IN ('admin', 'manager', 'agent')
                "#,
        )
        .bind(membership_id)
        .bind(agency_id)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "staff member {membership_id} not found in this agency"
            )));
        }
        Ok(())
    }
}
