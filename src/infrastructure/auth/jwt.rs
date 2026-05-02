use async_trait::async_trait;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use crate::{
    application::{errors::AppError, ports::auth_port::AuthPort},
    domain::auth::JwtClaims,
};

pub struct JwtAuth {
    pub secret: String,
    pub expiry_seconds: u64,
}

#[async_trait]
impl AuthPort for JwtAuth {
    async fn hash_password(&self, password: &str) -> Result<String, AppError> {
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::ExternalService(format!("argon2 hash: {e}")))
    }

    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AppError> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };
        let parsed =
            PasswordHash::new(hash).map_err(|e| AppError::ExternalService(e.to_string()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    fn sign_token(&self, claims: &JwtClaims) -> Result<String, AppError> {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AppError::ExternalService(format!("jwt sign: {e}")))
    }

    fn verify_token(&self, token: &str) -> Result<JwtClaims, AppError> {
        let mut v = Validation::new(Algorithm::HS256);
        v.validate_exp = true;
        decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &v,
        )
        .map(|d| d.claims)
        .map_err(|_| AppError::Unauthorised)
    }
}