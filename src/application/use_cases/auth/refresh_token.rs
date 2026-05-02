use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::auth_port::AuthPort},
    domain::auth::JwtClaims,
};

pub struct RefreshTokenUseCase {
    pub auth: Arc<dyn AuthPort>,
}

pub struct RefreshInput {
    pub refresh_token: String,
    pub expiry_seconds: u64,
}

pub struct RefreshOutput {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
}

impl RefreshTokenUseCase {
    pub fn new(auth: Arc<dyn AuthPort>) -> Self {
        Self { auth }
    }

    pub async fn execute(&self, input: RefreshInput) -> Result<RefreshOutput, AppError> {
        // Uses: AuthPort::verify_token
        let old = self.auth.verify_token(&input.refresh_token)?;

        let exp = OffsetDateTime::now_utc().unix_timestamp() as usize
            + input.expiry_seconds as usize;

        let new_claims = JwtClaims {
            sub: old.sub,
            agency_id: old.agency_id,
            role: old.role,
            jti: Uuid::new_v4().to_string(),
            exp,
        };

        // Uses: AuthPort::sign_token
        let access_token = self.auth.sign_token(&new_claims)?;

        Ok(RefreshOutput {
            access_token,
            token_type: "Bearer",
            expires_in: input.expiry_seconds,
        })
    }
}