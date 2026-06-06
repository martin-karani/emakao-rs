use crate::domain::{
    enums::{
        WorkOrderCategory, WorkOrderCommentAuthorType, WorkOrderPriority, WorkOrderReporterType,
        WorkOrderStatus,
    },
    maintenance::WorkOrderAttachment,
};
use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::{Date, OffsetDateTime};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ── Caretaker DTOs ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateCaretakerDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(length(min = 1, max = 100))]
    pub first_name: String,
    #[garde(length(min = 1, max = 100))]
    pub last_name: String,
    #[garde(length(min = 7, max = 20))]
    pub phone: Option<String>,
    #[garde(email)]
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateCaretakerDto {
    #[garde(length(min = 1, max = 100))]
    pub first_name: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub last_name: Option<String>,
    #[garde(length(min = 7, max = 20))]
    pub phone: Option<String>,
    #[garde(email)]
    pub email: Option<String>,
    #[garde(skip)]
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListCaretakersParams {
    pub property_id: Option<Uuid>,
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Work Order DTOs ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateWorkOrderDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub unit_id: Option<Uuid>,
    #[garde(length(min = 1, max = 200))]
    pub title: String,
    #[garde(length(max = 5000))]
    pub description: Option<String>,
    #[garde(skip)]
    pub category: WorkOrderCategory,
    #[garde(skip)]
    pub priority: WorkOrderPriority,
    /// Who is submitting — defaults to 'staff' when called from the staff API.
    /// Set to 'resident' / 'caretaker' via their respective portals.
    #[garde(skip)]
    pub reporter_type: Option<WorkOrderReporterType>,
    /// Populated only when reporter_type = resident
    #[garde(skip)]
    pub reporter_resident_id: Option<Uuid>,
    /// Populated only when reporter_type = caretaker
    #[garde(skip)]
    pub reporter_caretaker_id: Option<Uuid>,
    #[garde(skip)]
    pub vendor_id: Option<Uuid>,
    /// Staff user UUID to assign internally
    #[garde(skip)]
    pub assigned_to: Option<Uuid>,
    #[garde(skip)]
    pub assigned_caretaker_id: Option<Uuid>,
    #[garde(skip)]
    pub due_date: Option<Date>,
    #[garde(skip)]
    pub scheduled_at: Option<OffsetDateTime>,
    #[garde(skip)]
    pub estimated_cost_kes: Option<Decimal>,
    #[garde(skip)]
    pub is_tenant_visible: Option<bool>,
    #[garde(length(max = 5000))]
    pub internal_notes: Option<String>,
    /// Pre-uploaded S3 attachment metadata
    #[garde(skip)]
    pub attachments: Option<Vec<WorkOrderAttachment>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateWorkOrderDto {
    #[garde(skip)]
    pub status: Option<WorkOrderStatus>,
    #[garde(skip)]
    pub priority: Option<WorkOrderPriority>,
    #[garde(skip)]
    pub category: Option<WorkOrderCategory>,
    /// Some(Some(id)) = assign vendor, Some(None) = clear vendor
    #[garde(skip)]
    pub vendor_id: Option<Option<Uuid>>,
    /// Some(Some(id)) = assign staff, Some(None) = clear
    #[garde(skip)]
    pub assigned_to: Option<Option<Uuid>>,
    #[garde(skip)]
    pub assigned_caretaker_id: Option<Option<Uuid>>,
    #[garde(length(max = 5000))]
    pub description: Option<String>,
    #[garde(length(max = 5000))]
    pub internal_notes: Option<String>,
    #[garde(skip)]
    pub due_date: Option<Option<Date>>,
    #[garde(skip)]
    pub scheduled_at: Option<Option<OffsetDateTime>>,
    #[garde(skip)]
    pub started_at: Option<Option<OffsetDateTime>>,
    #[garde(skip)]
    pub completed_at: Option<Option<OffsetDateTime>>,
    #[garde(skip)]
    pub estimated_cost_kes: Option<Option<Decimal>>,
    #[garde(skip)]
    pub actual_cost_kes: Option<Option<Decimal>>,
    #[garde(skip)]
    pub is_tenant_visible: Option<bool>,
    /// Full replacement of the attachments list
    #[garde(skip)]
    pub attachments: Option<Vec<WorkOrderAttachment>>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListWorkOrdersParams {
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub status: Option<WorkOrderStatus>,
    pub priority: Option<WorkOrderPriority>,
    pub category: Option<WorkOrderCategory>,
    pub reporter_type: Option<WorkOrderReporterType>,
    pub assigned_caretaker_id: Option<Uuid>,
    pub reporter_resident_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Comment DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateWorkOrderCommentDto {
    /// Omit to create a top-level comment; set to reply to another comment
    #[garde(skip)]
    pub parent_comment_id: Option<Uuid>,
    #[garde(length(min = 1, max = 10_000))]
    pub body: String,
    /// true = only staff/caretakers see this (internal memo)
    #[garde(skip)]
    pub is_internal: Option<bool>,
    #[garde(skip)]
    pub author_type: Option<WorkOrderCommentAuthorType>,
    #[garde(skip)]
    pub author_resident_id: Option<Uuid>,
    #[garde(skip)]
    pub author_caretaker_id: Option<Uuid>,
    #[garde(skip)]
    pub attachments: Option<Vec<WorkOrderAttachment>>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListWorkOrderCommentsParams {
    /// Only return top-level comments when true; include replies when false (default)
    pub top_level_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
