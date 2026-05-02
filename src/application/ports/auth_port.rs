use async_trait::async_trait;

use crate::{
    application::errors::AppError,
    domain::auth::JwtClaims,
};

#[async_trait]
pub trait AuthPort: Send + Sync + 'static {
    async fn hash_password(&self, password: &str) -> Result<String, AppError>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AppError>;
    fn sign_token(&self, claims: &JwtClaims) -> Result<String, AppError>;
    fn verify_token(&self, token: &str) -> Result<JwtClaims, AppError>;
}