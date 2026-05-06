use crate::domain::enums::PortalType;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::use_cases::auth::login::LoginOutput;

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub agency_id: Uuid,
    pub agency_name: String,
    pub agency_slug: String,
    pub portal: PortalType,
    pub must_change_password: bool,
}

impl From<LoginOutput> for LoginResponse {
    fn from(value: LoginOutput) -> Self {
        Self {
            access_token: value.access_token,
            token_type: value.token_type,
            expires_in: value.expires_in,
            agency_id: value.agency_id,
            agency_name: value.agency_name,
            agency_slug: value.agency_slug,
            portal: value.portal,
            must_change_password: value.must_change_password,
        }
    }
}
