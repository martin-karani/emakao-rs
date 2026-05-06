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
        enums::WorkOrderStatus,
        maintenance::{UpdateWorkOrderCommand, WorkOrder},
    },
};

pub struct UpdateWorkOrderUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
    pub notifications: NotificationService,
}

pub struct UpdateWorkOrderInput {
    pub agency_id: Uuid,
    pub work_order_id: Uuid,
    pub status: Option<WorkOrderStatus>,
    pub vendor_id: Option<Option<Uuid>>,
    pub description: Option<String>,
    /// If provided, an email notification is sent to this address.
    pub resident_email: Option<String>,
    /// If provided, an SMS notification is sent to this number.
    pub resident_phone: Option<String>,
    /// Human-readable display name for the work order (e.g. "WO-0042").
    pub work_order_ref: Option<String>,
    /// Category label shown in the notification (e.g. "Plumbing").
    pub category: Option<String>,
    /// Vendor name shown in the email if a vendor is assigned.
    pub vendor_name: Option<String>,
    /// ISO 8601 date string shown if the work is scheduled.
    pub scheduled_at: Option<String>,
}

impl UpdateWorkOrderUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>, notifications: NotificationService) -> Self {
        Self {
            repo,
            notifications,
        }
    }

    pub async fn execute(&self, input: UpdateWorkOrderInput) -> Result<WorkOrder, AppError> {
        self.repo
            .find_by_id(input.agency_id, input.work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {}", input.work_order_id)))?;

        let updated = self
            .repo
            .update(UpdateWorkOrderCommand {
                id: input.work_order_id,
                agency_id: input.agency_id,
                status: input.status.clone(),
                vendor_id: input.vendor_id,
                description: input.description.clone(),
            })
            .await?;

        // Only notify when the status actually changed and we have a recipient.
        if input.status.is_some()
            && (input.resident_email.is_some() || input.resident_phone.is_some())
        {
            let status_label = updated.status;
            let work_order_id_str = input.work_order_id.to_string();
            let wo_ref = input
                .work_order_ref
                .as_deref()
                .unwrap_or(&work_order_id_str);
            let category = input.category.as_deref().unwrap_or("Maintenance");

            let ctx = serde_json::json!({
                "work_order_ref": wo_ref,
                "category":       category,
                "status":         status_label,
                "scheduled_at":   input.scheduled_at,
                "vendor_name":    input.vendor_name,
                "description":    input.description,
            });

            if let Some(email) = &input.resident_email {
                let _ = self
                    .notifications
                    .email(email.clone(), EmailTemplate::WorkOrderUpdate, ctx.clone())
                    .await;
            }

            if let Some(phone) = &input.resident_phone {
                let _ = self
                    .notifications
                    .sms(phone.clone(), SmsTemplate::WorkOrderUpdate, ctx)
                    .await;
            }
        }

        Ok(updated)
    }
}
