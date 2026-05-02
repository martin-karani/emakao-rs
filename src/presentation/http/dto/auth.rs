use garde::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginDto {
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 8))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterDto {
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 8))]
    pub password: String,
    /// Staff role assigned to this user: "admin" | "manager" | "agent"
    #[garde(length(min = 1, max = 50))]
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshDto {
    pub refresh_token: String,
}
