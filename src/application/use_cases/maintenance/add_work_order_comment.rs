use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::{
        enums::WorkOrderCommentAuthorType,
        maintenance::{CreateWorkOrderCommentCommand, WorkOrderAttachment, WorkOrderComment},
    },
};

pub struct AddWorkOrderCommentUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl AddWorkOrderCommentUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }
}

pub struct AddWorkOrderCommentInput {
    pub agency_id: Uuid,
    pub work_order_id: Uuid,

    // ── Threading ─────────────────────────────────────────────────────────────
    /// `None` = top-level comment. `Some(id)` = reply to that comment.
    pub parent_comment_id: Option<Uuid>,

    // ── Author ────────────────────────────────────────────────────────────────
    pub author_id: Uuid,
    pub author_type: WorkOrderCommentAuthorType,
    /// Populated only when author_type = Resident.
    pub author_resident_id: Option<Uuid>,
    /// Populated only when author_type = Caretaker.
    pub author_caretaker_id: Option<Uuid>,

    // ── Content ───────────────────────────────────────────────────────────────
    pub body: String,
    /// `true` = visible to staff and caretakers only; hidden from residents/owners.
    pub is_internal: bool,
    pub attachments: Vec<WorkOrderAttachment>,
}

impl AddWorkOrderCommentUseCase {
    pub async fn execute(
        &self,
        input: AddWorkOrderCommentInput,
    ) -> Result<WorkOrderComment, AppError> {
        // ── Validation ────────────────────────────────────────────────────────
        let body = input.body.trim().to_string();
        if body.is_empty() {
            return Err(AppError::Validation(
                "comment body must not be empty".into(),
            ));
        }
        if body.len() > 10_000 {
            return Err(AppError::Validation(
                "comment body must be 10 000 characters or fewer".into(),
            ));
        }

        // Caretakers and residents should not post internal comments.
        // Enforce that here so the handler doesn't need to remember.
        let is_internal = match input.author_type {
            WorkOrderCommentAuthorType::Resident | WorkOrderCommentAuthorType::Owner => false,
            _ => input.is_internal,
        };

        // Ensure the work order exists within this agency before commenting.
        self.repo
            .find_by_id(input.agency_id, input.work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {}", input.work_order_id)))?;

        // If this is a reply, verify the parent exists on the same work order.
        if let Some(parent_id) = input.parent_comment_id {
            let parent_exists = self
                .repo
                .find_comments(
                    input.work_order_id,
                    true, // include internal — we're just checking existence
                    false,
                    1,
                    0,
                )
                .await?
                .into_iter()
                .any(|c| c.id == parent_id);

            if !parent_exists {
                return Err(AppError::Validation(
                    "parent_comment_id does not belong to this work order".into(),
                ));
            }
        }

        // ── Persist ───────────────────────────────────────────────────────────
        let comment = self
            .repo
            .create_comment(CreateWorkOrderCommentCommand {
                work_order_id: input.work_order_id,
                parent_comment_id: input.parent_comment_id,
                author_id: input.author_id,
                author_type: input.author_type.clone(),
                author_resident_id: input.author_resident_id,
                author_caretaker_id: input.author_caretaker_id,
                body,
                is_internal,
                attachments: input.attachments,
            })
            .await?;

        // ── Activity log ──────────────────────────────────────────────────────
        let actor_type = author_type_label(&input.author_type);
        let event = if input.parent_comment_id.is_some() {
            "reply_added"
        } else {
            "comment_added"
        };

        self.repo
            .log_activity(
                input.work_order_id,
                input.author_id,
                actor_type,
                event,
                serde_json::json!({
                    "comment_id":  comment.id,
                    "is_internal": comment.is_internal,
                    "has_attachments": !comment.attachments.is_empty(),
                    "parent_comment_id": input.parent_comment_id,
                }),
            )
            .await
            .ok();

        Ok(comment)
    }
}

fn author_type_label(a: &WorkOrderCommentAuthorType) -> &'static str {
    match a {
        WorkOrderCommentAuthorType::Staff => "staff",
        WorkOrderCommentAuthorType::Resident => "resident",
        WorkOrderCommentAuthorType::Caretaker => "caretaker",
        WorkOrderCommentAuthorType::Owner => "owner",
        WorkOrderCommentAuthorType::Vendor => "vendor",
    }
}
