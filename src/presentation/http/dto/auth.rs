use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginDto {
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 8))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterDto {
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 8))]
    pub password: String,
    /// Staff role assigned to this user: "admin" | "manager" | "agent"
    #[garde(length(min = 1, max = 50))]
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshDto {
    pub refresh_token: String,
}

/// Staff login — requires agency_slug.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct StaffLoginDto {
    #[garde(length(min = 1))]
    pub agency_slug: String,
    /// Email address or Kenyan phone number (07xx / +2547xx).
    #[garde(length(min = 3))]
    pub contact: String,
    #[garde(length(min = 8))]
    pub password: String,
}

/// Portal login (resident / owner / vendor) — no agency_slug needed.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PortalLoginDto {
    #[garde(length(min = 3))]
    pub contact: String,
    #[garde(length(min = 8))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AcceptInviteDto {
    #[garde(length(min = 32, max = 32))]
    pub token: String,
    #[garde(length(min = 8))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordDto {
    #[garde(length(min = 8))]
    pub old_password: String,
    #[garde(length(min = 8))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ForgotPasswordDto {
    #[garde(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordDto {
    #[garde(length(min = 64, max = 64))] // raw 32-byte token encoded as 64 hex chars
    pub token: String,
    #[garde(length(min = 8, max = 128))]
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}
