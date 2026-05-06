use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{auth_port::AuthPort, auth_repository::AuthRepository},
    },
    domain::{
        auth::{ContactMethod, JwtClaims},
        enums::PortalType,
    },
};

pub struct LoginOutput {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub agency_id: Uuid,
    pub agency_name: String,
    pub agency_slug: String,
    pub portal: PortalType,
    pub must_change_password: bool,
}

//
// Requires agency_slug in the request body.
// Accepts email OR phone as the contact.

pub struct StaffLoginInput {
    pub agency_slug: String,
    pub contact: String,
    pub password: String,
    pub expiry_seconds: u64,
}

pub struct StaffLoginUseCase {
    pub repo: Arc<dyn AuthRepository>,
    pub auth: Arc<dyn AuthPort>,
}

impl StaffLoginUseCase {
    pub fn new(repo: Arc<dyn AuthRepository>, auth: Arc<dyn AuthPort>) -> Self {
        Self { repo, auth }
    }

    pub async fn execute(&self, input: StaffLoginInput) -> Result<LoginOutput, AppError> {
        let agency = self
            .repo
            .find_agency_by_slug(&input.agency_slug)
            .await?
            .ok_or(AppError::Unauthorised)?;

        let contact = ContactMethod::parse(&input.contact);

        let user = self
            .repo
            .find_staff_by_contact(agency.id, contact.value(), contact.type_str())
            .await?
            .ok_or(AppError::Unauthorised)?;

        if !user.is_active {
            return Err(AppError::Unauthorised);
        }

        let role = user.role.as_deref().unwrap_or("");
        if PortalType::from_role(role) != PortalType::Staff {
            return Err(AppError::Unauthorised);
        }

        if !self
            .auth
            .verify_password(&input.password, &user.password_hash)
            .await?
        {
            return Err(AppError::Unauthorised);
        }

        let _ = self.repo.update_last_login(user.id).await;

        let exp = expiry(input.expiry_seconds);
        let claims = JwtClaims {
            sub: user.id,
            agency_id: agency.id,
            role: role.to_owned(),
            portal: PortalType::Staff,
            jti: Uuid::new_v4().to_string(),
            exp,
        };

        tracing::info!(user_id = %user.id, agency = %agency.slug, "staff login");

        Ok(LoginOutput {
            access_token: self.auth.sign_token(&claims)?,
            token_type: "Bearer",
            expires_in: input.expiry_seconds,
            agency_id: agency.id,
            agency_name: agency.name,
            agency_slug: agency.slug,
            portal: PortalType::Staff,
            must_change_password: user.must_change_password,
        })
    }
}

//
// No agency_slug needed — resolved from portal_user_index by contact+portal.

pub struct PortalLoginInput {
    pub portal: PortalType,
    /// Raw email or phone
    pub contact: String,
    pub password: String,
    pub expiry_seconds: u64,
}

pub struct PortalLoginUseCase {
    pub repo: Arc<dyn AuthRepository>,
    pub auth: Arc<dyn AuthPort>,
}

impl PortalLoginUseCase {
    pub fn new(repo: Arc<dyn AuthRepository>, auth: Arc<dyn AuthPort>) -> Self {
        Self { repo, auth }
    }

    pub async fn execute(&self, input: PortalLoginInput) -> Result<LoginOutput, AppError> {
        let contact = ContactMethod::parse(&input.contact);

        let identity = self
            .repo
            .find_portal_identity(contact.value(), contact.type_str(), input.portal.as_str())
            .await?
            .ok_or(AppError::Unauthorised)?;

        let user = self
            .repo
            .find_user_by_id(identity.user_id)
            .await?
            .ok_or(AppError::Unauthorised)?;

        if !user.is_active {
            return Err(AppError::Forbidden(
                "Account not yet activated — check your email or SMS for the invite link.".into(),
            ));
        }

        let (role, mem_active) = self
            .repo
            .find_membership(user.id, identity.agency_id)
            .await?
            .ok_or(AppError::Unauthorised)?;

        if !mem_active {
            return Err(AppError::Unauthorised);
        }

        if PortalType::from_role(&role) != input.portal {
            return Err(AppError::Unauthorised);
        }

        if !self
            .auth
            .verify_password(&input.password, &user.password_hash)
            .await?
        {
            return Err(AppError::Unauthorised);
        }

        let _ = self.repo.update_last_login(user.id).await;

        let agency = self
            .repo
            .find_agency_by_id(identity.agency_id)
            .await?
            .ok_or(AppError::Unauthorised)?;

        let exp = expiry(input.expiry_seconds);
        let claims = JwtClaims {
            sub: user.id,
            agency_id: identity.agency_id,
            role: role.clone(),
            portal: input.portal,
            jti: Uuid::new_v4().to_string(),
            exp,
        };

        tracing::info!(
            user_id   = %user.id,
            agency_id = %identity.agency_id,
            portal    = %input.portal,
            "portal login"
        );

        Ok(LoginOutput {
            access_token: self.auth.sign_token(&claims)?,
            token_type: "Bearer",
            expires_in: input.expiry_seconds,
            agency_id: identity.agency_id,
            agency_name: agency.name,
            agency_slug: agency.slug,
            portal: input.portal,
            must_change_password: user.must_change_password,
        })
    }
}

fn expiry(seconds: u64) -> usize {
    OffsetDateTime::now_utc().unix_timestamp() as usize + seconds as usize
}
