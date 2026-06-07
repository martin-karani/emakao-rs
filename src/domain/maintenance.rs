use crate::domain::enums::{
    WorkOrderCategory, WorkOrderCommentAuthorType, WorkOrderPriority, WorkOrderReporterType,
    WorkOrderStatus,
};
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Attachment & Subtask (shared by work order and comment) ─────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkOrderAttachment {
    pub key: String,  // S3 object key
    pub url: String,  // pre-signed or public URL
    pub name: String, // original filename
    pub mime: String, // MIME type, e.g. "image/jpeg"
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkOrderSubtask {
    pub id: String,
    pub title: String,
    pub is_completed: bool,
}

// ── Caretaker ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct Caretaker {
    pub id: Uuid,
    pub property_id: Uuid,
    pub user_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub is_active: bool,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct CreateCaretakerCommand {
    pub property_id: Uuid,
    pub created_by: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
}

pub struct UpdateCaretakerCommand {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub is_active: Option<bool>,
}

// ── Work Order ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct WorkOrder {
    pub id: Uuid,
    pub property_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub code: String,

    // core
    pub title: String,
    pub description: Option<String>,
    pub category: WorkOrderCategory,
    pub status: WorkOrderStatus,
    pub priority: WorkOrderPriority,
    pub work_order_number: i32,

    // reporter
    pub reported_by: Uuid,
    pub reporter_type: WorkOrderReporterType,
    pub reporter_resident_id: Option<Uuid>,
    pub reporter_caretaker_id: Option<Uuid>,

    // assignment
    pub assigned_to: Option<Uuid>,
    pub assigned_caretaker_id: Option<Uuid>,

    // scheduling & cost
    pub due_date: Option<Date>,
    pub scheduled_at: Option<OffsetDateTime>,
    pub started_at: Option<OffsetDateTime>,
    pub completed_at: Option<OffsetDateTime>,
    pub estimated_cost_kes: Option<rust_decimal::Decimal>,
    pub actual_cost_kes: Option<rust_decimal::Decimal>,

    // visibility & content
    pub is_tenant_visible: bool,
    pub internal_notes: Option<String>,
    pub attachments: Vec<WorkOrderAttachment>,
    pub subtasks: Vec<WorkOrderSubtask>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct CreateWorkOrderCommand {
    pub property_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub reported_by: Uuid,
    pub reporter_type: WorkOrderReporterType,
    pub reporter_resident_id: Option<Uuid>,
    pub reporter_caretaker_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub category: WorkOrderCategory,
    pub priority: WorkOrderPriority,
    pub vendor_id: Option<Uuid>,
    pub assigned_to: Option<Uuid>,
    pub assigned_caretaker_id: Option<Uuid>,
    pub due_date: Option<Date>,
    pub scheduled_at: Option<OffsetDateTime>,
    pub estimated_cost_kes: Option<rust_decimal::Decimal>,
    pub is_tenant_visible: bool,
    pub internal_notes: Option<String>,
    pub attachments: Vec<WorkOrderAttachment>,
    pub subtasks: Vec<WorkOrderSubtask>,
}

pub struct UpdateWorkOrderCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub status: Option<WorkOrderStatus>,
    pub priority: Option<WorkOrderPriority>,
    pub category: Option<WorkOrderCategory>,
    pub vendor_id: Option<Option<Uuid>>, // Some(Some(id)) = set, Some(None) = clear
    pub assigned_to: Option<Option<Uuid>>,
    pub assigned_caretaker_id: Option<Option<Uuid>>,
    pub description: Option<String>,
    pub internal_notes: Option<String>,
    pub due_date: Option<Option<Date>>,
    pub scheduled_at: Option<Option<OffsetDateTime>>,
    pub started_at: Option<Option<OffsetDateTime>>,
    pub completed_at: Option<Option<OffsetDateTime>>,
    pub estimated_cost_kes: Option<Option<rust_decimal::Decimal>>,
    pub actual_cost_kes: Option<Option<rust_decimal::Decimal>>,
    pub is_tenant_visible: Option<bool>,
    /// Replaces the entire attachments array (append logic lives in use-case)
    pub attachments: Option<Vec<WorkOrderAttachment>>,
    /// Replaces the entire subtasks array
    pub subtasks: Option<Vec<WorkOrderSubtask>>,
}

// ── Work Order Comment ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct WorkOrderComment {
    pub id: Uuid,
    pub work_order_id: Uuid,
    pub parent_comment_id: Option<Uuid>,
    pub author_id: Uuid,
    pub author_type: WorkOrderCommentAuthorType,
    pub author_resident_id: Option<Uuid>,
    pub author_caretaker_id: Option<Uuid>,
    pub body: String,
    pub is_internal: bool,
    pub attachments: Vec<WorkOrderAttachment>,
    pub is_edited: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct CreateWorkOrderCommentCommand {
    pub work_order_id: Uuid,
    pub parent_comment_id: Option<Uuid>,
    pub author_id: Uuid,
    pub author_type: WorkOrderCommentAuthorType,
    pub author_resident_id: Option<Uuid>,
    pub author_caretaker_id: Option<Uuid>,
    pub body: String,
    pub is_internal: bool,
    pub attachments: Vec<WorkOrderAttachment>,
}

// ── Activity Log ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct WorkOrderActivity {
    pub id: Uuid,
    pub work_order_id: Uuid,
    pub actor_id: Uuid,
    pub actor_type: WorkOrderCommentAuthorType,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: OffsetDateTime,
}
