use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::accounting::{
            create_account::{CreateAccountInput, CreateAccountUseCase},
            get_trial_balance::GetTrialBalanceUseCase,
            get_vat_report::{GetVatReportInput, GetVatReportUseCase},
            list_accounts::{ListAccountsInput, ListAccountsUseCase},
            list_journal_entries::{ListJournalEntriesInput, ListJournalEntriesUseCase},
            post_journal_entry::{PostJournalEntryInput, PostJournalEntryUseCase},
            void_journal_entry::{VoidJournalEntryInput, VoidJournalEntryUseCase},
        },
    },
    domain::{
        accounting::{AccountType, JournalEntryStatus, JournalLine},
        auth::AuthenticatedUser,
    },
    infrastructure::db::accounting_repository_sqlx::PgAccountingRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            helpers::permission::check_permission,
            responses::accounting::{
                AccountResponse, JournalEntryResponse, TrialBalanceResponse, VatReportResponse,
            },
        },
    },
};

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListAccountsParams {
    pub account_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListJournalEntriesParams {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct VatReportParams {
    pub period_start: Date,
    pub period_end: Date,
}

fn default_limit() -> i64 {
    50
}

// ── Request bodies ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountDto {
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    #[serde(default)]
    pub vat_applicable: bool,
    pub vat_rate: Option<Decimal>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PostJournalEntryDto {
    pub reference: String,
    pub description: Option<String>,
    pub lines: Vec<JournalLine>,
    #[serde(default = "bool_true")]
    pub post_immediately: bool,
}

fn bool_true() -> bool {
    true
}

// ── Accounts ──────────────────────────────────────────────────────────────────

/// GET /api/v1/accounts
#[utoipa::path(get, path = "/api/v1/accounts", params(ListAccountsParams),
    responses((status=200,body=Vec<AccountResponse>),(status=401,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn list_accounts(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListAccountsParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let account_type = params
        .account_type
        .as_deref()
        .and_then(AccountType::from_str);
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    let accounts = ListAccountsUseCase { repo }
        .execute(ListAccountsInput {
            agency_id: ctx.agency.id,
            account_type,
            limit: params.limit,
            offset: params.offset,
        })
        .await?;
    Ok(Json(
        accounts
            .into_iter()
            .map(AccountResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// POST /api/v1/accounts
#[utoipa::path(post, path = "/api/v1/accounts", request_body=CreateAccountDto,
    responses((status=201,body=AccountResponse),(status=409,body=ErrorResponse),(status=401,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn create_account(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateAccountDto>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    let account = CreateAccountUseCase { repo }
        .execute(CreateAccountInput {
            agency_id: ctx.agency.id,
            code: dto.code,
            name: dto.name,
            account_type: dto.account_type,
            vat_applicable: dto.vat_applicable,
            vat_rate: dto.vat_rate,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(AccountResponse::from(account))))
}

/// DELETE /api/v1/accounts/:id
#[utoipa::path(delete, path = "/api/v1/accounts/{id}",
    params(("id"=Uuid, Path, description="Account UUID")),
    responses((status=204),(status=404,body=ErrorResponse),(status=401,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn delete_account(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    repo.delete_account(ctx.agency.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Journal entries ───────────────────────────────────────────────────────────

/// GET /api/v1/journal-entries
#[utoipa::path(get, path = "/api/v1/journal-entries", params(ListJournalEntriesParams),
    responses((status=200,body=Vec<JournalEntryResponse>),(status=401,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn list_journal_entries(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListJournalEntriesParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let status = params
        .status
        .as_deref()
        .and_then(JournalEntryStatus::from_str);
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    let entries = ListJournalEntriesUseCase { repo }
        .execute(ListJournalEntriesInput {
            agency_id: ctx.agency.id,
            status,
            limit: params.limit,
            offset: params.offset,
        })
        .await?;
    Ok(Json(
        entries
            .into_iter()
            .map(JournalEntryResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /api/v1/journal-entries/:id
#[utoipa::path(get, path = "/api/v1/journal-entries/{id}",
    params(("id"=Uuid,Path,description="Journal entry UUID")),
    responses((status=200,body=JournalEntryResponse),(status=404,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn get_journal_entry(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    let entry = repo
        .find_journal_entry_by_id(ctx.agency.id, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("journal entry {id}")))?;
    Ok(Json(JournalEntryResponse::from(entry)))
}

/// POST /api/v1/journal-entries
#[utoipa::path(post, path = "/api/v1/journal-entries", request_body=PostJournalEntryDto,
    responses((status=201,body=JournalEntryResponse),(status=422,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn post_journal_entry(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<PostJournalEntryDto>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    let entry = PostJournalEntryUseCase { repo }
        .execute(PostJournalEntryInput {
            agency_id: ctx.agency.id,
            reference: dto.reference,
            description: dto.description,
            lines: dto.lines,
            posted_by: user.user_id,
            post_immediately: dto.post_immediately,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(JournalEntryResponse::from(entry))))
}

/// POST /api/v1/journal-entries/:id/void
#[utoipa::path(post, path = "/api/v1/journal-entries/{id}/void",
    params(("id"=Uuid,Path,description="Journal entry UUID")),
    responses((status=200,body=JournalEntryResponse),(status=409,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn void_journal_entry(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    let entry = VoidJournalEntryUseCase { repo }
        .execute(VoidJournalEntryInput {
            agency_id: ctx.agency.id,
            entry_id: id,
            voided_by: user.user_id,
        })
        .await?;
    Ok(Json(JournalEntryResponse::from(entry)))
}

// ── Reports ───────────────────────────────────────────────────────────────────

/// GET /api/v1/accounting/trial-balance
#[utoipa::path(get, path = "/api/v1/accounting/trial-balance",
    responses((status=200,body=TrialBalanceResponse),(status=401,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn get_trial_balance(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    let tb = GetTrialBalanceUseCase { repo }
        .execute(ctx.agency.id)
        .await?;
    Ok(Json(TrialBalanceResponse::from(tb)))
}

/// GET /api/v1/accounting/vat-report?period_start=YYYY-MM-DD&period_end=YYYY-MM-DD
#[utoipa::path(get, path = "/api/v1/accounting/vat-report", params(VatReportParams),
    responses((status=200,body=VatReportResponse),(status=400,body=ErrorResponse)),
    tag="Accounting", security(("bearer_token"=[])))]
pub async fn get_vat_report(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<VatReportParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    let repo = Arc::new(PgAccountingRepo::new(ctx.pool));
    let report = GetVatReportUseCase { repo }
        .execute(GetVatReportInput {
            agency_id: ctx.agency.id,
            period_start: params.period_start,
            period_end: params.period_end,
        })
        .await?;
    Ok(Json(VatReportResponse::from(report)))
}

// ── Private helpers ───────────────────────────────────────────────────────────

use crate::application::ports::accounting_repository::AccountingRepository;
