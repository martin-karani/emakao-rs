use async_trait::async_trait;
use time::Date;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::{
        enums::InvoiceStatus,
        invoice::{Invoice, InvoiceLineItem},
    },
};

pub struct CreateInvoiceCommand {
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub line_items: Vec<InvoiceLineItem>,
    pub due_date: Date,
    pub notes: Option<String>,
    pub created_by: Uuid,
}

pub struct InvoiceFilter {
    pub property_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub status: Option<InvoiceStatus>,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
pub trait InvoiceRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        filter: InvoiceFilter,
    ) -> Result<Vec<Invoice>, AppError>;

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Invoice>, AppError>;

    async fn create(&self, cmd: CreateInvoiceCommand) -> Result<Invoice, AppError>;

    async fn update_status(&self, id: Uuid, status: InvoiceStatus) -> Result<Invoice, AppError>;

    async fn void(&self, id: Uuid) -> Result<Invoice, AppError>;
}
