use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::maintenance::WorkOrder,
};

// ── Get by UUID ───────────────────────────────────────────────────────────────

pub struct GetWorkOrderUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl GetWorkOrderUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    /// Fetch a work order by its internal UUID.
    /// Scoped to the agency so tenants cannot access other agencies' data.
    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<WorkOrder, AppError> {
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {id}")))
    }
}

// ── Get by human-readable code ────────────────────────────────────────────────

pub struct GetWorkOrderByCodeUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl GetWorkOrderByCodeUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    /// Fetch a work order by its human-readable code, e.g. `"MGRD-0042"`.
    ///
    /// Codes are normalised to uppercase before lookup so callers can pass
    /// `mgrd-0042`, `MGRD-0042`, or `Mgrd-0042` interchangeably.
    pub async fn execute(&self, agency_id: Uuid, code: &str) -> Result<WorkOrder, AppError> {
        let code_upper = code.trim().to_ascii_uppercase();
        if code_upper.is_empty() {
            return Err(AppError::Validation(
                "work order code must not be empty".into(),
            ));
        }
        self.repo
            .find_by_code(agency_id, &code_upper)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {code_upper}")))
    }
}

// ── Get orders for a caretaker ─────────────────────────────────────────────

pub struct GetWorkOrdersForCaretakerUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl GetWorkOrdersForCaretakerUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    /// Returns all work orders that a caretaker either submitted or is
    /// assigned to, across the properties they manage.
    pub async fn execute(
        &self,
        caretaker_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
        let limit = limit.clamp(1, 100);
        let offset = offset.max(0);
        self.repo
            .find_for_caretaker(caretaker_id, limit, offset)
            .await
    }
}

// ── Get orders visible to a resident ──────────────────────────────────────────

pub struct GetWorkOrdersForResidentUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl GetWorkOrdersForResidentUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    /// Returns work orders for units where the resident has an active lease,
    /// filtered to `is_tenant_visible = true`. Internal-only fields such as
    /// `internal_notes` are stripped by the response layer, not here.
    pub async fn execute(
        &self,
        resident_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
        let limit = limit.clamp(1, 100);
        let offset = offset.max(0);
        self.repo
            .find_for_resident(resident_id, limit, offset)
            .await
    }
}
