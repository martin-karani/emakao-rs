use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        notifications::{
            service::NotificationService,
            templates::{EmailTemplate, SmsTemplate},
        },
        ports::maintenance_repository::MaintenanceRepository,
    },
    domain::{
        enums::{WorkOrderCategory, WorkOrderPriority, WorkOrderReporterType},
        maintenance::{CreateWorkOrderCommand, WorkOrder, WorkOrderAttachment},
    },
};
use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

pub struct CreateWorkOrderUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
    pub notifications: NotificationService,
}

impl CreateWorkOrderUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>, notifications: NotificationService) -> Self {
        Self {
            repo,
            notifications,
        }
    }
}

pub struct CreateWorkOrderInput {
    // ── Identity ──────────────────────────────────────────────────────────────
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub unit_id: Option<Uuid>,

    // ── Reporter ──────────────────────────────────────────────────────────────
    /// The platform user UUID of whoever is making this call.
    pub reported_by: Uuid,
    /// Defaults to Staff when called from the staff API; overridden
    /// by caretaker/resident portal handlers before reaching this use case.
    pub reporter_type: WorkOrderReporterType,
    pub reporter_resident_id: Option<Uuid>,
    pub reporter_caretaker_id: Option<Uuid>,

    // ── Core fields ───────────────────────────────────────────────────────────
    pub title: String,
    pub description: Option<String>,
    pub category: WorkOrderCategory,
    pub priority: WorkOrderPriority,

    // ── Assignment ────────────────────────────────────────────────────────────
    pub vendor_id: Option<Uuid>,
    pub assigned_to: Option<Uuid>,
    pub assigned_caretaker_id: Option<Uuid>,

    // ── Scheduling & cost ─────────────────────────────────────────────────────
    pub due_date: Option<Date>,
    pub scheduled_at: Option<OffsetDateTime>,
    pub estimated_cost_kes: Option<Decimal>,

    // ── Visibility ────────────────────────────────────────────────────────────
    pub is_tenant_visible: bool,
    pub internal_notes: Option<String>,

    // ── Attachments ───────────────────────────────────────────────────────────
    pub attachments: Vec<WorkOrderAttachment>,

    // ── Notification recipients (optional) ────────────────────────────────────
    /// Send a creation notification to this resident email, if provided.
    pub notify_resident_email: Option<String>,
    /// Send a creation SMS to this resident phone, if provided.
    pub notify_resident_phone: Option<String>,
}

impl CreateWorkOrderUseCase {
    pub async fn execute(&self, input: CreateWorkOrderInput) -> Result<WorkOrder, AppError> {
        // ── Validation ────────────────────────────────────────────────────────
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::Validation(
                "work order title must not be empty".into(),
            ));
        }
        if title.len() > 200 {
            return Err(AppError::Validation(
                "work order title must be 200 characters or fewer".into(),
            ));
        }

        // A caretaker reporter must reference a known caretaker record.
        if input.reporter_type == WorkOrderReporterType::Caretaker
            && input.reporter_caretaker_id.is_none()
        {
            return Err(AppError::Validation(
                "reporter_caretaker_id is required when reporter_type is caretaker".into(),
            ));
        }

        // A resident reporter must reference a known resident record.
        if input.reporter_type == WorkOrderReporterType::Resident
            && input.reporter_resident_id.is_none()
        {
            return Err(AppError::Validation(
                "reporter_resident_id is required when reporter_type is resident".into(),
            ));
        }

        // ── Persist ───────────────────────────────────────────────────────────
        // The repo atomically increments the property's work_order_seq,
        // computes the code (e.g. "MGRD-0042"), and inserts the row.
        let order = self
            .repo
            .create(CreateWorkOrderCommand {
                property_id: input.property_id,
                unit_id: input.unit_id,
                reported_by: input.reported_by,
                reporter_type: input.reporter_type.clone(),
                reporter_resident_id: input.reporter_resident_id,
                reporter_caretaker_id: input.reporter_caretaker_id,
                title,
                description: input.description.clone(),
                category: input.category,
                priority: input.priority,
                vendor_id: input.vendor_id,
                assigned_to: input.assigned_to,
                assigned_caretaker_id: input.assigned_caretaker_id,
                due_date: input.due_date,
                scheduled_at: input.scheduled_at,
                estimated_cost_kes: input.estimated_cost_kes,
                is_tenant_visible: input.is_tenant_visible,
                internal_notes: input.internal_notes.clone(),
                attachments: input.attachments,
            })
            .await?;

        // ── Activity log ──────────────────────────────────────────────────────
        let actor_type = reporter_type_label(&input.reporter_type);
        self.repo
            .log_activity(
                order.id,
                input.reported_by,
                actor_type,
                "created",
                serde_json::json!({
                    "code":     order.code,
                    "title":    order.title,
                    "priority": format!("{:?}", order.priority),
                    "category": format!("{:?}", order.category),
                }),
            )
            .await
            .ok(); // never fail the main flow on a log write

        // ── Optional notification ─────────────────────────────────────────────
        // Notify the resident/owner that their work order has been received.
        let should_notify = order.is_tenant_visible
            && (input.notify_resident_email.is_some() || input.notify_resident_phone.is_some());

        if should_notify {
            let ctx = serde_json::json!({
                "code":        order.code,
                "title":       order.title,
                "category":    format!("{:?}", order.category),
                "priority":    format!("{:?}", order.priority),
                "description": order.description,
            });

            if let Some(email) = &input.notify_resident_email {
                let _ = self
                    .notifications
                    .email(email.clone(), EmailTemplate::WorkOrderUpdate, ctx.clone())
                    .await;
            }
            if let Some(phone) = &input.notify_resident_phone {
                let _ = self
                    .notifications
                    .sms(phone.clone(), SmsTemplate::WorkOrderUpdate, ctx)
                    .await;
            }
        }

        tracing::info!(
            work_order_id = %order.id,
            code           = %order.code,
            reporter_type  = ?input.reporter_type,
            "work order created"
        );

        Ok(order)
    }
}

fn reporter_type_label(r: &WorkOrderReporterType) -> &'static str {
    match r {
        WorkOrderReporterType::Staff => "staff",
        WorkOrderReporterType::Resident => "resident",
        WorkOrderReporterType::Caretaker => "caretaker",
        WorkOrderReporterType::Owner => "owner",
        WorkOrderReporterType::Vendor => "vendor",
    }
}
