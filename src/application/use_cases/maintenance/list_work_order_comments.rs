use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::maintenance::WorkOrderComment,
};

pub struct ListWorkOrderCommentsUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl ListWorkOrderCommentsUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }
}

pub struct ListWorkOrderCommentsInput {
    pub agency_id: Uuid,
    pub work_order_id: Uuid,
    /// `true` = staff/caretaker portal (sees internal comments).
    /// `false` = resident/owner portal (internal comments hidden).
    pub include_internal: bool,
    /// `true` = return only top-level comments (no replies).
    /// The caller fetches replies separately via `find_comment_replies`.
    pub top_level_only: bool,
    pub limit: i64,
    pub offset: i64,
}

impl ListWorkOrderCommentsUseCase {
    pub async fn execute(
        &self,
        input: ListWorkOrderCommentsInput,
    ) -> Result<Vec<WorkOrderComment>, AppError> {
        // Verify the work order is accessible within this agency.
        self.repo
            .find_by_id(input.agency_id, input.work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {}", input.work_order_id)))?;

        let limit = input.limit.clamp(1, 200);
        let offset = input.offset.max(0);

        self.repo
            .find_comments(
                input.work_order_id,
                input.include_internal,
                input.top_level_only,
                limit,
                offset,
            )
            .await
    }
}

// ── Replies ───────────────────────────────────────────────────────────────────

pub struct ListCommentRepliesUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl ListCommentRepliesUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        work_order_id: Uuid,
        parent_comment_id: Uuid,
        include_internal: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrderComment>, AppError> {
        // Scope check: ensure the work order belongs to this agency.
        self.repo
            .find_by_id(agency_id, work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {work_order_id}")))?;

        let limit = limit.clamp(1, 200);
        let offset = offset.max(0);

        self.repo
            .find_comment_replies(parent_comment_id, include_internal, limit, offset)
            .await
    }
}
