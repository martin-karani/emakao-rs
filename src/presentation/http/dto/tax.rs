use crate::domain::tax::{TaxObligationStatus, TaxObligationType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct TaxObligationFilterParams {
    pub owner_id: Option<Uuid>,
    pub property_id: Option<Uuid>,
    pub obligation_type: Option<TaxObligationType>,
    pub status: Option<TaxObligationStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct FileTaxObligationDto {
    pub kra_ack_number: String,
}

#[derive(Debug, Deserialize)]
pub struct MarkTaxPaidDto {
    pub kra_prn: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyKraPinDto {
    pub pin: String,
}
