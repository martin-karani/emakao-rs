// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added WorkOrderAttachmentResponse, WorkOrderCommentResponse, WorkOrderActivityResponse
//   - Replaced domain types in WorkOrderResponse, WorkOrderPublicResponse, WorkOrderCommentResponse
//   - Added rfc3339 serialization to all OffsetDateTime fields
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{
    enums::{
        WorkOrderCategory, WorkOrderCommentAuthorType, WorkOrderPriority, WorkOrderReporterType,
        WorkOrderStatus,
    },
    maintenance::{Caretaker, WorkOrder, WorkOrderActivity, WorkOrderAttachment, WorkOrderComment},
};

// ── Attachment & Subtask ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkOrderAttachmentResponse {
    pub key: String,
    pub url: String,
    pub name: String,
    pub mime: String,
    pub size_bytes: u64,
}

impl From<WorkOrderAttachment> for WorkOrderAttachmentResponse {
    fn from(a: WorkOrderAttachment) -> Self {
        Self {
            key: a.key,
            url: a.url,
            name: a.name,
            mime: a.mime,
            size_bytes: a.size_bytes,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkOrderSubtaskResponse {
    pub id: String,
    pub title: String,
    pub is_completed: bool,
}

impl From<crate::domain::maintenance::WorkOrderSubtask> for WorkOrderSubtaskResponse {
    fn from(s: crate::domain::maintenance::WorkOrderSubtask) -> Self {
        Self {
            id: s.id,
            title: s.title,
            is_completed: s.is_completed,
        }
    }
}

// ── Caretaker ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct CaretakerResponse {
    pub id: Uuid,
    pub property_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<Caretaker> for CaretakerResponse {
    fn from(c: Caretaker) -> Self {
        Self {
            id: c.id,
            property_id: c.property_id,
            user_id: c.user_id,
            first_name: c.first_name,
            last_name: c.last_name,
            phone: c.phone,
            email: c.email,
            is_active: c.is_active,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

// ── Work Order ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkOrderResponse {
    pub id: Uuid,
    pub code: String,
    pub work_order_number: i32,
    pub property_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_id: Option<Uuid>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub category: WorkOrderCategory,
    pub status: WorkOrderStatus,
    pub priority: WorkOrderPriority,
    pub reported_by: Uuid,
    pub reporter_type: WorkOrderReporterType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporter_resident_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporter_caretaker_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_caretaker_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<Date>,
    #[serde(with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_kes: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_cost_kes: Option<Decimal>,
    pub is_tenant_visible: bool,
    /// internal_notes is omitted from the struct here; expose only on staff routes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_notes: Option<String>,
    pub attachments: Vec<WorkOrderAttachmentResponse>,
    pub subtasks: Vec<WorkOrderSubtaskResponse>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<WorkOrder> for WorkOrderResponse {
    fn from(w: WorkOrder) -> Self {
        Self {
            id: w.id,
            code: w.code,
            work_order_number: w.work_order_number,
            property_id: w.property_id,
            unit_id: w.unit_id,
            vendor_id: w.vendor_id,
            title: w.title,
            description: w.description,
            category: w.category,
            status: w.status,
            priority: w.priority,
            reported_by: w.reported_by,
            reporter_type: w.reporter_type,
            reporter_resident_id: w.reporter_resident_id,
            reporter_caretaker_id: w.reporter_caretaker_id,
            assigned_to: w.assigned_to,
            assigned_caretaker_id: w.assigned_caretaker_id,
            due_date: w.due_date,
            scheduled_at: w.scheduled_at,
            started_at: w.started_at,
            completed_at: w.completed_at,
            estimated_cost_kes: w.estimated_cost_kes,
            actual_cost_kes: w.actual_cost_kes,
            is_tenant_visible: w.is_tenant_visible,
            internal_notes: w.internal_notes,
            attachments: w.attachments.into_iter().map(WorkOrderAttachmentResponse::from).collect(),
            subtasks: w.subtasks.into_iter().map(WorkOrderSubtaskResponse::from).collect(),
            created_at: w.created_at,
            updated_at: w.updated_at,
        }
    }
}

/// Resident/owner-facing response — strips internal fields
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkOrderPublicResponse {
    pub id: Uuid,
    pub property_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<Uuid>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub category: WorkOrderCategory,
    pub status: WorkOrderStatus,
    pub priority: WorkOrderPriority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<Date>,
    #[serde(with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<OffsetDateTime>,
    pub attachments: Vec<WorkOrderAttachmentResponse>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<WorkOrder> for WorkOrderPublicResponse {
    fn from(w: WorkOrder) -> Self {
        Self {
            id: w.id,
            property_id: w.property_id,
            unit_id: w.unit_id,
            title: w.title,
            description: w.description,
            category: w.category,
            status: w.status,
            priority: w.priority,
            due_date: w.due_date,
            scheduled_at: w.scheduled_at,
            attachments: w.attachments.into_iter().map(WorkOrderAttachmentResponse::from).collect(),
            created_at: w.created_at,
            updated_at: w.updated_at,
        }
    }
}

// ── Comment ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkOrderCommentResponse {
    pub id: Uuid,
    pub work_order_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_comment_id: Option<Uuid>,
    pub author_id: Uuid,
    pub author_type: WorkOrderCommentAuthorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_resident_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_caretaker_id: Option<Uuid>,
    pub body: String,
    pub is_internal: bool,
    pub attachments: Vec<WorkOrderAttachmentResponse>,
    pub is_edited: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<WorkOrderComment> for WorkOrderCommentResponse {
    fn from(c: WorkOrderComment) -> Self {
        Self {
            id: c.id,
            work_order_id: c.work_order_id,
            parent_comment_id: c.parent_comment_id,
            author_id: c.author_id,
            author_type: c.author_type,
            author_resident_id: c.author_resident_id,
            author_caretaker_id: c.author_caretaker_id,
            body: c.body,
            is_internal: c.is_internal,
            attachments: c.attachments.into_iter().map(WorkOrderAttachmentResponse::from).collect(),
            is_edited: c.is_edited,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

// ── Activity ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkOrderActivityResponse {
    pub id: Uuid,
    pub work_order_id: Uuid,
    pub actor_id: Uuid,
    pub actor_type: WorkOrderCommentAuthorType,
    pub event_type: String,
    pub payload: serde_json::Value,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<WorkOrderActivity> for WorkOrderActivityResponse {
    fn from(a: WorkOrderActivity) -> Self {
        Self {
            id: a.id,
            work_order_id: a.work_order_id,
            actor_id: a.actor_id,
            actor_type: a.actor_type,
            event_type: a.event_type,
            payload: a.payload,
            created_at: a.created_at,
        }
    }
}
