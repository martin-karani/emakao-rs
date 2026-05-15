// src/application/ports/inspection_repository.rs

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::{
        enums::{InspectionStatus, InspectionType},
        inspection::{Inspection, InspectionItem},
    },
};

// ── Commands ──────────────────────────────────────────────────────────────────

pub struct CreateInspectionCommand {
    pub property_id: Uuid,
    pub unit_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub inspection_type: InspectionType,
    pub scheduled_at: OffsetDateTime,
    pub created_by: Uuid,
}

pub struct UpdateInspectionCommand {
    pub id: Uuid,
    /// Update status (e.g. scheduled → completed)
    pub status: Option<InspectionStatus>,
    pub scheduled_at: Option<OffsetDateTime>,
    pub completed_at: Option<OffsetDateTime>,
    pub conducted_by: Option<Option<Uuid>>,
    pub items: Option<Vec<InspectionItem>>,
    pub summary_notes: Option<String>,
}

// ── Filter ────────────────────────────────────────────────────────────────────

pub struct InspectionFilter {
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub status: Option<InspectionStatus>,
    pub inspection_type: Option<InspectionType>,
    pub limit: i64,
    pub offset: i64,
}

// ── Port ──────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait InspectionRepository: Send + Sync + 'static {
    async fn find_all(&self, filter: InspectionFilter) -> Result<Vec<Inspection>, AppError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Inspection>, AppError>;

    async fn create(&self, cmd: CreateInspectionCommand) -> Result<Inspection, AppError>;

    async fn update(&self, cmd: UpdateInspectionCommand) -> Result<Inspection, AppError>;

    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}
