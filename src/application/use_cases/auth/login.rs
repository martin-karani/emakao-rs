use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{auth_port::AuthPort, auth_repository::AuthRepository},
    },
    domain::auth::JwtClaims,
};

pub struct LoginUseCase {
    pub repo: Arc<dyn AuthRepository>,
    pub auth: Arc<dyn AuthPort>,
}

pub struct LoginInput {
    pub agency_id: Uuid,
    pub email: String,
    pub password: String,
    pub expiry_seconds: u64,
}

pub struct LoginOutput {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
}

impl LoginUseCase {
    pub fn new(repo: Arc<dyn AuthRepository>, auth: Arc<dyn AuthPort>) -> Self {
        Self { repo, auth }
    }

    pub async fn execute(&self, input: LoginInput) -> Result<LoginOutput, AppError> {
        // Uses: AuthRepository::find_by_email
        let user = self
            .repo
            .find_by_email(input.agency_id, &input.email)
            .await?
            .ok_or(AppError::Unauthorised)?;

        if !user.is_active {
            return Err(AppError::Unauthorised);
        }

        // Uses: AuthPort::verify_password
        let valid = self.auth.verify_password(&input.password, &user.password_hash).await?;
        if !valid {
            return Err(AppError::Unauthorised);
        }

        let exp =
            OffsetDateTime::now_utc().unix_timestamp() as usize + input.expiry_seconds as usize;

        let claims = JwtClaims {
            sub: user.id,
            agency_id: input.agency_id,
            role: user.role,
            jti: Uuid::new_v4().to_string(),
            exp,
        };

        // Uses: AuthPort::sign_token
        let access_token = self.auth.sign_token(&claims)?;

        Ok(LoginOutput { access_token, token_type: "Bearer", expires_in: input.expiry_seconds })
    }
}