use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::enums::{WorkOrderCategory, WorkOrderPriority, WorkOrderReporterType, WorkOrderStatus},
    domain::maintenance::{
        Caretaker, CreateCaretakerCommand, CreateWorkOrderCommand, CreateWorkOrderCommentCommand,
        UpdateCaretakerCommand, UpdateWorkOrderCommand, WorkOrder, WorkOrderActivity,
        WorkOrderComment,
    },
};

#[async_trait]
pub trait MaintenanceRepository: Send + Sync + 'static {
    // ── Caretakers ────────────────────────────────────────────────────────────

    async fn find_caretakers(
        &self,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Caretaker>, AppError>;

    async fn find_caretaker_by_id(&self, id: Uuid) -> Result<Option<Caretaker>, AppError>;

    async fn create_caretaker(&self, cmd: CreateCaretakerCommand) -> Result<Caretaker, AppError>;

    async fn update_caretaker(&self, cmd: UpdateCaretakerCommand) -> Result<Caretaker, AppError>;

    // ── Work Orders ───────────────────────────────────────────────────────────

    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        unit_id: Option<Uuid>,
        status: Option<WorkOrderStatus>,
        priority: Option<WorkOrderPriority>,
        category: Option<WorkOrderCategory>,
        reporter_type: Option<WorkOrderReporterType>,
        assigned_caretaker_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError>;

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<WorkOrder>, AppError>;

    /// Work orders visible to a specific resident (their unit + is_tenant_visible = true)
    async fn find_for_resident(
        &self,
        resident_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError>;

    /// Work orders assigned to or reported by a caretaker
    async fn find_for_caretaker(
        &self,
        caretaker_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError>;

    async fn find_work_orders_by_vendor_id(
        &self,
        vendor_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError>;

    async fn find_by_code(
        &self,
        agency_id: Uuid,
        code: &str,
    ) -> Result<Option<WorkOrder>, AppError>;

    async fn create(&self, cmd: CreateWorkOrderCommand) -> Result<WorkOrder, AppError>;

    async fn update(&self, cmd: UpdateWorkOrderCommand) -> Result<WorkOrder, AppError>;

    // ── Comments ──────────────────────────────────────────────────────────────

    async fn find_comments(
        &self,
        work_order_id: Uuid,
        include_internal: bool, // false for resident/owner portals
        top_level_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrderComment>, AppError>;

    async fn find_comment_replies(
        &self,
        parent_comment_id: Uuid,
        include_internal: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrderComment>, AppError>;

    async fn create_comment(
        &self,
        cmd: CreateWorkOrderCommentCommand,
    ) -> Result<WorkOrderComment, AppError>;

    // ── Activity Log ──────────────────────────────────────────────────────────

    async fn find_activity(
        &self,
        work_order_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrderActivity>, AppError>;

    async fn log_activity(
        &self,
        work_order_id: Uuid,
        actor_id: Uuid,
        actor_type: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), AppError>;
}
