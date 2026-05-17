//
// Upload a file and persist it to the `documents` table in one atomic operation:
//   1. Validate MIME type and file size (shared helpers — same policy everywhere).
//   2. Build the canonical S3 key via `document_s3_key`.
//   3. PUT the bytes to S3.
//   4. INSERT a row into `documents` and return the created record.
//
// The key is generated *here* (not in the handler) so that the use-case owns
// the full document life-cycle.  The handler is responsible only for parsing
// multipart and mapping the domain result to an HTTP response.

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        helpers::upload::{document_s3_key, ext_from_mime, validate_mime, validate_size},
        ports::{document_repository::DocumentRepository, storage_port::StoragePort},
    },
    domain::document::{CreateDocumentCommand, Document, DocumentType},
};

// ── Input ─────────────────────────────────────────────────────────────────────

pub struct UploadDocumentInput {
    pub agency_id: Uuid,
    pub uploaded_by: Uuid,

    pub file_name: String,

    /// MIME type as reported by the multipart field.
    /// Validated against the allow-list before the upload proceeds.
    pub mime_type: String,

    pub file_bytes: Vec<u8>,
    pub document_type: DocumentType,
    pub title: Option<String>,
    pub notes: Option<String>,

    // ── Optional entity linkage ───────────────────────────────────────────────
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub work_order_id: Option<Uuid>,
}

pub struct UploadDocumentUseCase {
    pub repo: Arc<dyn DocumentRepository>,
    pub storage: Arc<dyn StoragePort>,
}

impl UploadDocumentUseCase {
    pub async fn execute(&self, input: UploadDocumentInput) -> Result<Document, AppError> {
        // ── Validate before touching S3 ───────────────────────────────────────
        validate_mime(&input.mime_type)?;
        validate_size(&input.file_bytes)?;

        // ── Build canonical S3 key ────────────────────────────────────────────
        // Always derive the extension from the MIME type (not the filename) so
        // the stored key is trustworthy and cannot be spoofed by the client.
        let ext = ext_from_mime(&input.mime_type);
        let s3_key = document_s3_key(input.agency_id, ext);
        let size_bytes = input.file_bytes.len() as i64;

        // ── Upload to S3 ──────────────────────────────────────────────────────
        self.storage
            .upload(&s3_key, input.file_bytes, &input.mime_type)
            .await
            .map_err(|e| AppError::ExternalService(format!("S3 upload failed: {e}")))?;

        // ── Persist to DB ─────────────────────────────────────────────────────
        self.repo
            .create(CreateDocumentCommand {
                agency_id: input.agency_id,
                s3_key,
                file_name: input.file_name,
                mime_type: input.mime_type,
                size_bytes,
                document_type: input.document_type,
                title: input.title,
                notes: input.notes,
                property_id: input.property_id,
                unit_id: input.unit_id,
                resident_id: input.resident_id,
                agreement_id: input.agreement_id,
                work_order_id: input.work_order_id,
                uploaded_by: input.uploaded_by,
            })
            .await
    }
}
