// src/application/ports/document_repository.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::document::{CreateDocumentCommand, Document, DocumentFilter},
};

#[async_trait]
pub trait DocumentRepository: Send + Sync + 'static {
    async fn find_all(&self, filter: DocumentFilter) -> Result<Vec<Document>, AppError>;
    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Document>, AppError>;
    async fn create(&self, cmd: CreateDocumentCommand) -> Result<Document, AppError>;
    async fn delete(&self, agency_id: Uuid, id: Uuid) -> Result<String, AppError>;
    async fn count_for_agency(&self, agency_id: Uuid) -> Result<i64, AppError>;
}
