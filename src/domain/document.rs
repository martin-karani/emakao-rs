// src/domain/document.rs
//
// Document management domain. Each document is a file stored in S3 and
// catalogued in the `documents` table with metadata and entity linkage.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

// ── Entity ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Document {
    pub id: Uuid,
    pub agency_id: Uuid,

    // S3
    pub s3_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,

    // Classification
    pub document_type: DocumentType,
    pub title: Option<String>,
    pub notes: Option<String>,

    // Entity linkage (all optional — a document can attach to any one entity)
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub work_order_id: Option<Uuid>,

    pub uploaded_by: Uuid,
    pub created_at: OffsetDateTime,
}

// ── Classification enum ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    LeaseAgreement,
    Noc, // No-Objection Certificate
    InspectionForm,
    UtilityBill,
    Receipt,
    IdDocument,
    Photo,
    Other,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LeaseAgreement => "lease_agreement",
            Self::Noc => "noc",
            Self::InspectionForm => "inspection_form",
            Self::UtilityBill => "utility_bill",
            Self::Receipt => "receipt",
            Self::IdDocument => "id_document",
            Self::Photo => "photo",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "lease_agreement" => Some(Self::LeaseAgreement),
            "noc" => Some(Self::Noc),
            "inspection_form" => Some(Self::InspectionForm),
            "utility_bill" => Some(Self::UtilityBill),
            "receipt" => Some(Self::Receipt),
            "id_document" => Some(Self::IdDocument),
            "photo" => Some(Self::Photo),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

pub struct CreateDocumentCommand {
    pub agency_id: Uuid,
    pub s3_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub document_type: DocumentType,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub work_order_id: Option<Uuid>,
    pub uploaded_by: Uuid,
}

pub struct DocumentFilter {
    pub agency_id: Uuid,
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub work_order_id: Option<Uuid>,
    pub document_type: Option<DocumentType>,
    pub limit: i64,
    pub offset: i64,
}
