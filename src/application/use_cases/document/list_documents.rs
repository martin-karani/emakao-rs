// src/application/use_cases/document/list_documents.rs

use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::document_repository::DocumentRepository},
    domain::document::{Document, DocumentFilter, DocumentType},
};

pub struct ListDocumentsInput {
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

pub struct ListDocumentsUseCase {
    pub repo: Arc<dyn DocumentRepository>,
}

impl ListDocumentsUseCase {
    pub async fn execute(&self, input: ListDocumentsInput) -> Result<Vec<Document>, AppError> {
        self.repo
            .find_all(DocumentFilter {
                agency_id: input.agency_id,
                property_id: input.property_id,
                unit_id: input.unit_id,
                resident_id: input.resident_id,
                agreement_id: input.agreement_id,
                work_order_id: input.work_order_id,
                document_type: input.document_type,
                limit: input.limit.min(100),
                offset: input.offset,
            })
            .await
    }
}

// ── Get ───────────────────────────────────────────────────────────────────────

pub struct GetDocumentUseCase {
    pub repo: Arc<dyn DocumentRepository>,
}

impl GetDocumentUseCase {
    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<Document, AppError> {
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("document {id}")))
    }
}

// ── Delete ────────────────────────────────────────────────────────────────────

use crate::application::ports::storage_port::StoragePort;

pub struct DeleteDocumentUseCase {
    pub repo: Arc<dyn DocumentRepository>,
    pub storage: Arc<dyn StoragePort>,
}

impl DeleteDocumentUseCase {
    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError> {
        // Fetch first so we have the S3 key.
        let s3_key = self.repo.delete(agency_id, id).await?;

        // Best-effort S3 delete — log but don't fail the request.
        if let Err(e) = self.storage.delete(&s3_key).await {
            tracing::warn!(s3_key = %s3_key, error = %e, "S3 delete failed after DB delete");
        }

        Ok(())
    }
}
