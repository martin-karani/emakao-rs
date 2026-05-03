use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agency {
    pub id:            Uuid,
    pub name:          String,
    pub slug:          String,
    pub schema_name:   String,
    pub country_code:  String,
    pub currency_code: String,
    pub fga_store_id:  Option<String>,
    pub status:        String,
}

/// Injected by `resolve_agency_context` into every authenticated request's extensions.
///
/// Portal type is NOT here — it lives in `AuthenticatedUser.portal` so a
/// single `ResolvedAgency` can serve requests from any portal type on the
/// same agency.
#[derive(Clone, Debug)]
pub struct ResolvedAgency {
    pub id:           Uuid,
    pub name:         String,
    pub slug:         String,
    pub schema_name:  String,
    pub fga_store_id: Option<String>,
}