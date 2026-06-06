// src/application/ports/unit_repository.rs
//
// Repository port for Unit CRUD operations.
// Commands live here (not in the domain) following the pattern used by
// other repositories in this codebase (e.g. invoice_repository.rs).

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::property::{CreateUnitCommand, Unit, UpdateUnitCommand},
};

#[async_trait]
pub trait UnitRepository: Send + Sync + 'static {
    /// Return all units that belong to a property, ordered by unit_number.
    async fn find_all_for_property(&self, property_id: Uuid) -> Result<Vec<Unit>, AppError>;

    /// Return a single unit by its UUID, or None if not found.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Unit>, AppError>;

    /// Return the property_id that owns a given unit (for permission resolution).
    async fn property_id_for_unit(&self, unit_id: Uuid) -> Result<Option<Uuid>, AppError>;

    /// Create a single unit.
    async fn create(&self, cmd: CreateUnitCommand) -> Result<Unit, AppError>;

    /// Create multiple units in a single transaction.
    /// Rolls back if any unit violates the unique (property_id, unit_number)
    /// constraint or any other DB invariant.
    async fn create_batch(&self, cmds: Vec<CreateUnitCommand>) -> Result<Vec<Unit>, AppError>;

    /// Update mutable fields of a unit.
    async fn update(&self, cmd: UpdateUnitCommand) -> Result<Unit, AppError>;

    /// Hard-delete a unit.
    /// Callers MUST check `has_active_agreement` first and return 409 if true.
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;

    /// Return true when the unit has at least one agreement with status = 'active'.
    async fn has_active_agreement(&self, unit_id: Uuid) -> Result<bool, AppError>;
}
