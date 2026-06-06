use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
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
        ports::resident_repository::ResidentRepository,
        use_cases::invoice::{
            create_invoice::{CreateInvoiceInput, CreateInvoiceUseCase},
            get_invoice::GetInvoiceUseCase,
            list_invoices::{ListInvoicesInput, ListInvoicesUseCase},
            update_invoice_status::{UpdateInvoiceStatusInput, UpdateInvoiceStatusUseCase},
        },
    },
    domain::{auth::AuthenticatedUser, enums::InvoiceStatus, invoice::InvoiceLineItem},
    infrastructure::{
        db::{invoice_repository_sqlx::PgInvoiceRepo, resident_repository_sqlx::PgResidentRepo},
        notifications::dispatcher::Recipient,
    },
    presentation::{
        app_state::AppState, extractors::AgencyContext, http::responses::invoice::InvoiceResponse,
    },
};

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateInvoiceDto {
    #[garde(skip)]
    pub property_id: Uuid,
    /// Owner UUID — links the invoice to a tax obligation for MRI purposes.
    #[garde(skip)]
    pub owner_id: Option<Uuid>,
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
    /// Optional line classification used for tax computation.
    /// Accepted values: `"rent"` | `"management_fee"` | `"utility"` |
    ///                  `"deposit"` | `"late_fee"` | `"other"`
    /// Defaults to `None` when omitted (treated as non-tax line).
    #[garde(skip)]
    pub line_type: Option<String>,
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
        (status = 200, description = "Invoice found",  body = InvoiceResponse),
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
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateInvoiceDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    // ── Map DTO line items → domain line items (now includes line_type) ────────
    let line_items = dto
        .line_items
        .into_iter()
        .map(|li| InvoiceLineItem {
            description: li.description,
            quantity: li.quantity,
            unit_price_kes: li.unit_price_kes,
            total_kes: li.quantity * li.unit_price_kes,
            line_type: li.line_type, // ← NEW: pass through classification
        })
        .collect();

    let repo = std::sync::Arc::new(PgInvoiceRepo::new(ctx.pool));
    let uc = CreateInvoiceUseCase::new(repo, state.customisation().workflow.clone());

    let invoice = uc
        .execute(CreateInvoiceInput {
            agency_id: ctx.agency.id,
            owner_id: dto.owner_id, // ← NEW: for MRI obligation linkage
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

/// POST /api/v1/invoices/:id/notify
#[utoipa::path(
    post, path = "/api/v1/invoices/{id}/notify",
    params(("id" = Uuid, Path, description = "Invoice UUID")),
    responses(
        (status = 200, description = "Notification sent"),
        (status = 404, description = "Invoice or Resident not found"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Invoices", security(("bearer_token" = []))
)]
pub async fn notify_invoice(
    state: axum::extract::State<crate::presentation::app_state::AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = std::sync::Arc::new(PgInvoiceRepo::new(ctx.pool.clone()));
    let uc = GetInvoiceUseCase::new(repo);
    let invoice = uc.execute(ctx.agency.id, id).await?;

    let resident_id = invoice.resident_id.ok_or_else(|| {
        AppError::Validation("Invoice must be linked to a resident to send notifications".into())
    })?;

    // Fetch resident contact info
    let resident_repo = PgResidentRepo::new(ctx.pool.clone());
    let resident = resident_repo
        .find_by_id(ctx.agency.id, resident_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Resident {} not found", resident_id)))?;

    // Dispatch notification
    let context = serde_json::json!({
        "tenant_name": format!("{} {}", resident.first_name, resident.last_name),
        "invoice_number": invoice.id.to_string().split('-').next().unwrap_or(""),
        "amount": invoice.total_kes,
        "due_date": invoice.due_date,
        "line_items": invoice.line_items,
    });

    let recipient = Recipient {
        phone: resident.phone,
        email: resident.email,
        whatsapp: None,
    };

    let settings = state
        .customisation()
        .settings
        .get_or_load(ctx.agency.id, &ctx.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    state
        .customisation()
        .notifications
        .dispatch(
            ctx.agency.id,
            Some(invoice.property_id),
            "invoice.new",
            recipient,
            context,
            &settings.communication,
        )
        .await
        .map_err(|e| AppError::InternalServer(format!("Failed to dispatch notification: {}", e)))?;

    Ok(StatusCode::OK)
}

/// GET /api/v1/invoices/:id/print
#[utoipa::path(
    get, path = "/api/v1/invoices/{id}/print",
    params(("id" = Uuid, Path, description = "Invoice UUID")),
    responses(
        (status = 200, description = "HTML invoice", body = String),
        (status = 404, description = "Not found"),
    ),
    tag = "Invoices", security(("bearer_token" = []))
)]
pub async fn print_invoice(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = std::sync::Arc::new(PgInvoiceRepo::new(ctx.pool.clone()));
    let uc = GetInvoiceUseCase::new(repo);
    let invoice = uc.execute(ctx.agency.id, id).await?;

    let settings = state
        .customisation()
        .settings
        .get_or_load(ctx.agency.id, &ctx.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    // Render using the agency's custom template or system default
    let tmpl_str = settings
        .branding
        .invoice_template
        .clone()
        .unwrap_or_else(|| {
            r#"
        <html>
        <head><style>body { font-family: sans-serif; padding: 40px; }</style></head>
        <body>
            <h1>Invoice #{{ invoice_number }}</h1>
            <p>Date: {{ created_at }}</p>
            <p>Due Date: {{ due_date }}</p>
            <hr/>
            <table width="100%">
                <thead><tr><th>Description</th><th>Qty</th><th>Price</th><th>Total</th></tr></thead>
                <tbody>
                {% for item in line_items %}
                    <tr>
                        <td>{{ item.description }}</td>
                        <td>{{ item.quantity }}</td>
                        <td>{{ item.unit_price_kes }}</td>
                        <td>{{ item.total_kes }}</td>
                    </tr>
                {% endfor %}
                </tbody>
            </table>
            <hr/>
            <h3>Total: KES {{ total_kes }}</h3>
        </body>
        </html>
        "#
            .to_string()
        });

    let context = serde_json::json!({
        "invoice_number": invoice.invoice_number,
        "created_at": invoice.created_at,
        "due_date": invoice.due_date,
        "line_items": invoice.line_items,
        "total_kes": invoice.total_kes,
        "agency_name": ctx.agency.name,
    });

    let html = state
        .jinja
        .template_from_str(&tmpl_str)
        .map_err(|e| AppError::InternalServer(format!("Template error: {}", e)))?
        .render(context)
        .map_err(|e| AppError::InternalServer(format!("Render error: {}", e)))?;

    Ok(([(header::CONTENT_TYPE, "text/html")], html))
}
#[utoipa::path(
    patch, path = "/api/v1/invoices/{id}/status",
    params(("id" = Uuid, Path, description = "Invoice UUID")),
    request_body = UpdateInvoiceStatusDto,
    responses(
        (status = 200, description = "Status updated",  body = InvoiceResponse),
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
