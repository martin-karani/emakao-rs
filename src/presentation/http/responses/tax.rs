use serde::Serialize;
use uuid::Uuid;
use time::{Date, OffsetDateTime};
use crate::domain::tax::{TaxObligation, TaxComplianceSummary};

#[derive(Debug, Serialize)]
pub struct TaxObligationResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub owner_id: Uuid,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub obligation_type: String,
    pub tax_period: String,
    pub gross_amount_kes: String,
    pub tax_kes: String,
    pub tax_rate: String,
    pub due_date: String,
    pub status: String,
    pub kra_prn: Option<String>,
    pub kra_ack_number: Option<String>,
    pub filed_at: Option<OffsetDateTime>,
    pub remitted_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<TaxObligation> for TaxObligationResponse {
    fn from(o: TaxObligation) -> Self {
        Self {
            id: o.id,
            agency_id: o.agency_id,
            owner_id: o.owner_id,
            property_id: o.property_id,
            agreement_id: o.agreement_id,
            obligation_type: format!("{:?}", o.obligation_type).to_lowercase(),
            tax_period: o.tax_period.label(),
            gross_amount_kes: o.gross_amount_kes.to_string(),
            tax_kes: o.tax_kes.to_string(),
            tax_rate: o.tax_rate.to_string(),
            due_date: o.due_date.to_string(),
            status: format!("{:?}", o.status).to_lowercase(),
            kra_prn: o.kra_prn,
            kra_ack_number: o.kra_ack_number,
            filed_at: o.filed_at,
            remitted_at: o.remitted_at,
            created_at: o.created_at,
            updated_at: o.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TaxComplianceSummaryResponse {
    pub total_mri_kes: String,
    pub total_vat_kes: String,
    pub total_wht_kes: String,
    pub obligations_total: u64,
    pub obligations_filed: u64,
    pub obligations_paid: u64,
    pub obligations_overdue: u64,
    pub mri_due_date: String,
    pub vat_due_date: String,
}

impl From<TaxComplianceSummary> for TaxComplianceSummaryResponse {
    fn from(s: TaxComplianceSummary) -> Self {
        Self {
            total_mri_kes: s.total_mri_kes.to_string(),
            total_vat_kes: s.total_vat_kes.to_string(),
            total_wht_kes: s.total_wht_kes.to_string(),
            obligations_total: s.obligations_total,
            obligations_filed: s.obligations_filed,
            obligations_paid: s.obligations_paid,
            obligations_overdue: s.obligations_overdue,
            mri_due_date: s.mri_due_date.to_string(),
            vat_due_date: s.vat_due_date.to_string(),
        }
    }
}
