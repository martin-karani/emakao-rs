// src/presentation/http/dto/document.rs
//
// Query parameters for listing documents.
// Upload itself is multipart and parsed directly in the handler.

use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListDocumentsParams {
    /// Filter by property.
    pub property_id: Option<Uuid>,
    /// Filter by unit.
    pub unit_id: Option<Uuid>,
    /// Filter by resident.
    pub resident_id: Option<Uuid>,
    /// Filter by agreement.
    pub agreement_id: Option<Uuid>,
    /// Filter by work order.
    pub work_order_id: Option<Uuid>,
    /// Filter by document type (e.g. `lease_agreement`, `noc`, `photo`, …).
    pub document_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}
