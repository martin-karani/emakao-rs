// src/presentation/http/responses/document.rs

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::storage_port::StoragePort},
    domain::document::{Document, DocumentType},
};

/// HTTP response body for a single persisted document.
///
/// `download_url` is a 15-minute presigned GET URL generated at response time.
/// Clients must re-fetch the document (or call a dedicated presign endpoint)
/// when the URL expires — do not cache it past its TTL.
///
/// `s3_key` is intentionally excluded from serialisation; it is an internal
/// storage detail that callers do not need.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub agency_id: Uuid,

    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,

    pub document_type: DocumentType,
    pub title: Option<String>,
    pub notes: Option<String>,

    // Entity linkage
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub work_order_id: Option<Uuid>,

    pub uploaded_by: Uuid,
    pub created_at: OffsetDateTime,

    /// 15-minute presigned S3 GET URL.  Use this to render or download the
    /// file directly from storage — the API does not proxy file bytes.
    pub download_url: String,
}

impl DocumentResponse {
    /// Build a `DocumentResponse` from a `Document` domain object, generating
    /// a presigned download URL via the shared `StoragePort`.
    ///
    /// Call this for single-document responses.  For lists, prefer
    /// `from_documents` so the presign calls are fanned out concurrently.
    pub async fn from_document(doc: Document, storage: &dyn StoragePort) -> Result<Self, AppError> {
        let download_url = storage.get_url(&doc.s3_key).await?;
        Ok(Self::assemble(doc, download_url))
    }

    /// Build a `Vec<DocumentResponse>` from a slice of `Document`s,
    /// generating all presigned URLs concurrently.
    ///
    /// Uses `try_join_all` so a single presign failure bubbles up as an error.
    /// Presigning with the AWS SDK v1 is pure local HMAC computation (no
    /// network call), so fanning out N calls is cheap.
    pub async fn from_documents(
        docs: Vec<Document>,
        storage: &dyn StoragePort,
    ) -> Result<Vec<Self>, AppError> {
        futures::future::try_join_all(
            docs.into_iter()
                .map(|doc| DocumentResponse::from_document(doc, storage)),
        )
        .await
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn assemble(d: Document, download_url: String) -> Self {
        Self {
            id: d.id,
            agency_id: d.agency_id,
            file_name: d.file_name,
            mime_type: d.mime_type,
            size_bytes: d.size_bytes,
            document_type: d.document_type,
            title: d.title,
            notes: d.notes,
            property_id: d.property_id,
            unit_id: d.unit_id,
            resident_id: d.resident_id,
            agreement_id: d.agreement_id,
            work_order_id: d.work_order_id,
            uploaded_by: d.uploaded_by,
            created_at: d.created_at,
            download_url,
        }
    }
}
