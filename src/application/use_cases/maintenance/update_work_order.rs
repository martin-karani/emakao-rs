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
        enums::{WorkOrderCategory, WorkOrderPriority, WorkOrderStatus},
        maintenance::{UpdateWorkOrderCommand, WorkOrder, WorkOrderAttachment},
    },
};
use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};

pub struct UpdateWorkOrderUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
    pub notifications: NotificationService,
}

impl UpdateWorkOrderUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>, notifications: NotificationService) -> Self {
        Self {
            repo,
            notifications,
        }
    }
}

pub struct UpdateWorkOrderInput {
    // ── Identity ──────────────────────────────────────────────────────────────
    pub agency_id: Uuid,
    pub work_order_id: Uuid,
    /// Platform user UUID making this change (written to the activity log).
    pub actor_id: Uuid,
    /// Role label for the activity log ("staff", "caretaker", etc.).
    pub actor_type: String,

    // ── Fields that may change ────────────────────────────────────────────────
    pub status: Option<WorkOrderStatus>,
    pub priority: Option<WorkOrderPriority>,
    pub category: Option<WorkOrderCategory>,
    /// `Some(Some(id))` = assign vendor, `Some(None)` = clear vendor assignment.
    pub vendor_id: Option<Option<Uuid>>,
    /// `Some(Some(id))` = assign staff, `Some(None)` = clear.
    pub assigned_to: Option<Option<Uuid>>,
    /// `Some(Some(id))` = assign caretaker, `Some(None)` = clear.
    pub assigned_caretaker_id: Option<Option<Uuid>>,
    pub description: Option<String>,
    pub internal_notes: Option<String>,
    pub due_date: Option<Option<Date>>,
    pub scheduled_at: Option<Option<OffsetDateTime>>,
    pub started_at: Option<Option<OffsetDateTime>>,
    pub completed_at: Option<Option<OffsetDateTime>>,
    pub estimated_cost_kes: Option<Option<Decimal>>,
    pub actual_cost_kes: Option<Option<Decimal>>,
    pub is_tenant_visible: Option<bool>,
    /// Full replacement of the attachments array (append logic lives in the handler).
    pub attachments: Option<Vec<WorkOrderAttachment>>,

    // ── Notification recipients (optional) ────────────────────────────────────
    pub notify_resident_email: Option<String>,
    pub notify_resident_phone: Option<String>,
}

impl UpdateWorkOrderUseCase {
    pub async fn execute(&self, input: UpdateWorkOrderInput) -> Result<WorkOrder, AppError> {
        // ── 1. Load current state for diff ────────────────────────────────────
        let before = self
            .repo
            .find_by_id(input.agency_id, input.work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {}", input.work_order_id)))?;

        // ── 2. Persist the update ─────────────────────────────────────────────
        let after = self
            .repo
            .update(UpdateWorkOrderCommand {
                id: input.work_order_id,
                agency_id: input.agency_id,
                status: input.status.clone(),
                priority: input.priority.clone(),
                category: input.category.clone(),
                vendor_id: input.vendor_id,
                assigned_to: input.assigned_to,
                assigned_caretaker_id: input.assigned_caretaker_id,
                description: input.description.clone(),
                internal_notes: input.internal_notes.clone(),
                due_date: input.due_date,
                scheduled_at: input.scheduled_at,
                started_at: input.started_at,
                completed_at: input.completed_at,
                estimated_cost_kes: input.estimated_cost_kes,
                actual_cost_kes: input.actual_cost_kes,
                is_tenant_visible: input.is_tenant_visible,
                attachments: input.attachments,
            })
            .await?;

        // ── 3. Diff-based activity log ────────────────────────────────────────
        // Write one log entry per meaningful field change so the timeline is
        // granular enough to be useful.

        if let Some(ref new_status) = input.status {
            if before.status != *new_status {
                self.repo
                    .log_activity(
                        after.id,
                        input.actor_id,
                        &input.actor_type,
                        "status_changed",
                        serde_json::json!({
                            "from": format!("{:?}", before.status),
                            "to":   format!("{:?}", new_status),
                        }),
                    )
                    .await
                    .ok();
            }
        }

        if let Some(ref new_priority) = input.priority {
            if before.priority != *new_priority {
                self.repo
                    .log_activity(
                        after.id,
                        input.actor_id,
                        &input.actor_type,
                        "priority_changed",
                        serde_json::json!({
                            "from": format!("{:?}", before.priority),
                            "to":   format!("{:?}", new_priority),
                        }),
                    )
                    .await
                    .ok();
            }
        }

        if let Some(Some(vendor_id)) = input.vendor_id {
            if before.vendor_id != Some(vendor_id) {
                self.repo
                    .log_activity(
                        after.id,
                        input.actor_id,
                        &input.actor_type,
                        "vendor_assigned",
                        serde_json::json!({ "vendor_id": vendor_id }),
                    )
                    .await
                    .ok();
            }
        } else if let Some(None) = input.vendor_id {
            if before.vendor_id.is_some() {
                self.repo
                    .log_activity(
                        after.id,
                        input.actor_id,
                        &input.actor_type,
                        "vendor_removed",
                        serde_json::json!({}),
                    )
                    .await
                    .ok();
            }
        }

        if let Some(Some(caretaker_id)) = input.assigned_caretaker_id {
            if before.assigned_caretaker_id != Some(caretaker_id) {
                self.repo
                    .log_activity(
                        after.id,
                        input.actor_id,
                        &input.actor_type,
                        "caretaker_assigned",
                        serde_json::json!({ "caretaker_id": caretaker_id }),
                    )
                    .await
                    .ok();
            }
        }

        if let Some(ref new_scheduled_at) = input.scheduled_at {
            if before.scheduled_at != *new_scheduled_at {
                self.repo
                    .log_activity(
                        after.id,
                        input.actor_id,
                        &input.actor_type,
                        "scheduled",
                        serde_json::json!({ "scheduled_at": new_scheduled_at }),
                    )
                    .await
                    .ok();
            }
        }

        if input.actual_cost_kes.is_some() && before.actual_cost_kes != after.actual_cost_kes {
            self.repo
                .log_activity(
                    after.id,
                    input.actor_id,
                    &input.actor_type,
                    "cost_updated",
                    serde_json::json!({
                        "estimated_kes": after.estimated_cost_kes,
                        "actual_kes":    after.actual_cost_kes,
                    }),
                )
                .await
                .ok();
        }

        // ── 4. Notify residents when status changed and order is tenant-visible ─
        let status_changed = input
            .status
            .as_ref()
            .map(|s| before.status != *s)
            .unwrap_or(false);

        let should_notify = status_changed
            && after.is_tenant_visible
            && (input.notify_resident_email.is_some() || input.notify_resident_phone.is_some());

        if should_notify {
            let ctx = serde_json::json!({
                "code":         after.code,
                "title":        after.title,
                "category":     format!("{:?}", after.category),
                "status":       format!("{:?}", after.status),
                "scheduled_at": after.scheduled_at,
                "description":  after.description,
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
            work_order_id = %after.id,
            code          = %after.code,
            actor         = %input.actor_id,
            "work order updated"
        );

        Ok(after)
    }
}
