use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum ContactMethod {
    Email(String),
    Phone(String), // E.164 — "+254712345678"
}

impl ContactMethod {
    pub fn value(&self) -> &str {
        match self {
            Self::Email(v) | Self::Phone(v) => v.as_str(),
        }
    }

    pub fn type_str(&self) -> &'static str {
        match self {
            Self::Email(_) => "email",
            Self::Phone(_) => "phone",
        }
    }

    /// Phone heuristic: no '@', leading '+' or '0', or all digits.
    /// Normalises Kenyan shorthand: 07xx → +2547xx, 2547xx → +2547xx.
    pub fn parse(raw: &str) -> Self {
        let s = raw.trim();
        if s.contains('@') {
            return Self::Email(s.to_lowercase());
        }
        // Strip formatting characters
        let digits: String = s
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '+')
            .collect();
        let e164 = if digits.starts_with('0') {
            format!("+254{}", &digits[1..])
        } else if digits.starts_with("254") {
            format!("+{}", digits)
        } else {
            digits
        };
        Self::Phone(e164)
    }
}

// ── PortalType ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PortalType {
    Staff,
    Resident,
    Owner,
    Vendor,
}

impl PortalType {
    /// Single source of truth: role string → portal.
    pub fn from_role(role: &str) -> Self {
        match role {
            "admin" | "manager" | "agent" | "platform_admin" => Self::Staff,
            "resident" => Self::Resident,
            "owner" => Self::Owner,
            "vendor" => Self::Vendor,
            _ => Self::Staff,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Staff => "staff",
            Self::Resident => "resident",
            Self::Owner => "owner",
            Self::Vendor => "vendor",
        }
    }

    /// Subdomain prefix this portal lives on, e.g. "residents" → residents.emakao.co.ke
    pub fn subdomain(self) -> &'static str {
        match self {
            Self::Staff => "app",
            Self::Resident => "residents",
            Self::Owner => "owners",
            Self::Vendor => "vendors",
        }
    }
}

impl std::fmt::Display for PortalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid,
    pub agency_id: Uuid,
    pub role: String,
    pub portal: PortalType,
    pub jti: String,
    pub exp: usize,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub agency_id: Uuid,
    pub role: String,
    pub portal: PortalType,
}

#[derive(Clone, Debug)]
pub struct StoredUser {
    pub id: Uuid,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub password_hash: String,
    pub is_active: bool,
    pub must_change_password: bool,
    pub agency_id: Option<Uuid>,
    pub role: Option<String>,
}

impl StoredUser {
    pub fn portal(&self) -> PortalType {
        self.role
            .as_deref()
            .map(PortalType::from_role)
            .unwrap_or(PortalType::Staff)
    }
}
