use serde::{Deserialize, Serialize};

use super::property::Unit;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitWithChildren {
    pub unit: Unit,
    pub children: Vec<Unit>,
}
