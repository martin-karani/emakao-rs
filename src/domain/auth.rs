use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid,
    pub agency_id: Uuid,
    pub role: String,
    pub jti: String,
    pub exp: usize,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub agency_id: Uuid,
    pub role: String,
}

/// Row returned from the users table — used only in auth flows.
#[derive(Clone, Debug)]
pub struct StoredUser {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
}
