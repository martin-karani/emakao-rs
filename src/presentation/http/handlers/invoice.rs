use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::invoice::{
            create_invoice::{CreateInvoiceInput, CreateInvoiceUseCase},
            get_invoice::GetInvoiceUseCase,
            list_invoices::{ListInvoicesInput, ListInvoicesUseCase},
            update_invoice_status::{UpdateInvoiceStatusInput, UpdateInvoiceStatusUseCase},
        },
    },
    domain::{auth::AuthenticatedUser, enums::InvoiceStatus, invoice::InvoiceLineItem},
    infrastructure::db::invoice_repository_sqlx::PgInvoiceRepo,
    presentation::{extractors::AgencyContext, http::responses::invoice::InvoiceResponse},
};

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateInvoiceDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub agreement_id: Option<Uuid>,
    #[garde(skip)]
    pub resident_id: Option<Uuid>,
    #[garde(length(min = 1))]
    pub line_items: Vec<InvoiceLineItemDto>,
    #[garde(skip)]
    pub due_date: Date,
    #[garde(length(max = 2000))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct InvoiceLineItemDto {
    #[garde(length(min = 1, max = 200))]
    pub description: String,
    #[garde(skip)]
    pub quantity: Decimal,
    #[garde(skip)]
    pub unit_price_kes: Decimal,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateInvoiceStatusDto {
    #[garde(skip)]
    pub status: InvoiceStatus,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListInvoicesParams {
    pub property_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub status: Option<InvoiceStatus>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/invoices
#[utoipa::path(
    get, path = "/api/v1/invoices",
    params(ListInvoicesParams),
    responses(
        (status = 200, description = "Invoice list", body = Vec<InvoiceResponse>),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Invoices", security(("bearer_token" = []))
)]
pub async fn list_invoices(
    ctx: AgencyContext,
    Query(params): Query<ListInvoicesParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = std::sync::Arc::new(PgInvoiceRepo::new(ctx.pool));
    let uc = ListInvoicesUseCase::new(repo);

    let invoices = uc
        .execute(ListInvoicesInput {
            agency_id: ctx.agency.id,
            property_id: params.property_id,
            agreement_id: params.agreement_id,
            resident_id: params.resident_id,
            status: params.status,
            limit: params.limit,
            offset: params.offset,
        })
        .await?;

    Ok(Json(
        invoices
            .into_iter()
            .map(InvoiceResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /api/v1/invoices/:id
#[utoipa::path(
    get, path = "/api/v1/invoices/{id}",
    params(("id" = Uuid, Path, description = "Invoice UUID")),
    responses(
        (status = 200, description = "Invoice found", body = InvoiceResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Invoices", security(("bearer_token" = []))
)]
pub async fn get_invoice(
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = std::sync::Arc::new(PgInvoiceRepo::new(ctx.pool));
    let uc = GetInvoiceUseCase::new(repo);

    let invoice = uc.execute(ctx.agency.id, id).await?;
    Ok(Json(InvoiceResponse::from(invoice)))
}

/// POST /api/v1/invoices
#[utoipa::path(
    post, path = "/api/v1/invoices",
    request_body = CreateInvoiceDto,
    responses(
        (status = 201, description = "Invoice created", body = InvoiceResponse),
        (status = 422, description = "Validation error"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Invoices", security(("bearer_token" = []))
)]
pub async fn create_invoice(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateInvoiceDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let line_items = dto
        .line_items
        .into_iter()
        .map(|li| InvoiceLineItem {
            description: li.description,
            quantity: li.quantity,
            unit_price_kes: li.unit_price_kes,
            total_kes: li.quantity * li.unit_price_kes,
        })
        .collect();

    let repo = std::sync::Arc::new(PgInvoiceRepo::new(ctx.pool));
    let uc = CreateInvoiceUseCase::new(repo);

    let invoice = uc
        .execute(CreateInvoiceInput {
            agency_id: ctx.agency.id,
            property_id: dto.property_id,
            agreement_id: dto.agreement_id,
            resident_id: dto.resident_id,
            line_items,
            due_date: dto.due_date,
            notes: dto.notes,
            created_by: user.user_id,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(InvoiceResponse::from(invoice))))
}

/// PATCH /api/v1/invoices/:id/status
#[utoipa::path(
    patch, path = "/api/v1/invoices/{id}/status",
    params(("id" = Uuid, Path, description = "Invoice UUID")),
    request_body = UpdateInvoiceStatusDto,
    responses(
        (status = 200, description = "Status updated", body = InvoiceResponse),
        (status = 404, description = "Not found"),
        (status = 422, description = "Invalid status transition"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Invoices", security(("bearer_token" = []))
)]
pub async fn update_invoice_status(
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateInvoiceStatusDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = std::sync::Arc::new(PgInvoiceRepo::new(ctx.pool));
    let uc = UpdateInvoiceStatusUseCase::new(repo);

    let invoice = uc
        .execute(UpdateInvoiceStatusInput {
            agency_id: ctx.agency.id,
            id,
            status: dto.status,
        })
        .await?;

    Ok(Json(InvoiceResponse::from(invoice)))
}
