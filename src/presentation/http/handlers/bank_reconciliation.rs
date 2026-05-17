//
// Routes:
//   GET  /api/v1/bank-statements               — list statements
//   GET  /api/v1/bank-statements/:id           — get one with all lines
//   POST /api/v1/bank-statements               — import a statement + lines
//   POST /api/v1/bank-statements/:id/lines/:line_id/match    — match line → journal entry
//   POST /api/v1/bank-statements/:id/lines/:line_id/unmatch  — clear match
//   GET  /api/v1/bank-statements/:id/reconciliation          — summary report

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
        ports::bank_reconciliation_repository::BankReconciliationRepository,
        use_cases::bank_reconciliation::{
            get_reconciliation_report::GetReconciliationReportUseCase,
            get_statement::GetStatementUseCase,
            import_statement::{
                ImportStatementInput, ImportStatementLineInput2, ImportStatementUseCase,
            },
            list_statements::{ListStatementsInput, ListStatementsUseCase},
            match_line::{MatchLineInput, MatchLineUseCase},
            unmatch_line::{UnmatchLineInput, UnmatchLineUseCase},
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::bank_reconciliation_repository_sqlx::PgBankReconciliationRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            helpers::permission::check_permission,
            responses::bank_reconciliation::{BankStatementResponse, ReconciliationReportResponse},
        },
    },
};

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListStatementsParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

// ── Request bodies ────────────────────────────────────────────────────────────

/// A single line in an imported bank statement.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportStatementLineDto {
    pub value_date: Date,
    pub description: String,
    /// Positive = credit / money in.  Negative = debit / money out.
    pub amount: Decimal,
    pub reference: Option<String>,
}

/// Import a bank statement with all its transaction lines.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportStatementDto {
    pub bank_name: String,
    pub account_number: String,
    pub statement_date: Date,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub lines: Vec<ImportStatementLineDto>,
}

/// Match a statement line to an existing posted journal entry.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MatchLineDto {
    pub journal_entry_id: Uuid,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/bank-statements
#[utoipa::path(
    get, path = "/api/v1/bank-statements",
    params(ListStatementsParams),
    responses(
        (status = 200, description = "List of bank statements (lines omitted)", body = Vec<BankStatementResponse>),
        (status = 401, description = "Unauthorised", body = ErrorResponse),
    ),
    tag = "BankReconciliation", security(("bearer_token" = []))
)]
pub async fn list_statements(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListStatementsParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo: Arc<dyn BankReconciliationRepository> =
        Arc::new(PgBankReconciliationRepo::new(ctx.pool));
    let stmts = ListStatementsUseCase { repo }
        .execute(ListStatementsInput {
            agency_id: ctx.agency.id,
            limit: params.limit.clamp(1, 100),
            offset: params.offset.max(0),
        })
        .await?;

    Ok(Json(
        stmts
            .into_iter()
            .map(BankStatementResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /api/v1/bank-statements/:id
#[utoipa::path(
    get, path = "/api/v1/bank-statements/{id}",
    params(("id" = Uuid, Path, description = "Statement UUID")),
    responses(
        (status = 200, description = "Statement with all lines", body = BankStatementResponse),
        (status = 404, description = "Not found",               body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "BankReconciliation", security(("bearer_token" = []))
)]
pub async fn get_statement(
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

    let repo: Arc<dyn BankReconciliationRepository> =
        Arc::new(PgBankReconciliationRepo::new(ctx.pool));
    let stmt = GetStatementUseCase { repo }
        .execute(ctx.agency.id, id)
        .await?;

    Ok(Json(BankStatementResponse::from(stmt)))
}

/// POST /api/v1/bank-statements
#[utoipa::path(
    post, path = "/api/v1/bank-statements",
    request_body = ImportStatementDto,
    responses(
        (status = 201, description = "Statement imported", body = BankStatementResponse),
        (status = 422, description = "Validation error",   body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "BankReconciliation", security(("bearer_token" = []))
)]
pub async fn import_statement(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<ImportStatementDto>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo: Arc<dyn BankReconciliationRepository> =
        Arc::new(PgBankReconciliationRepo::new(ctx.pool));
    let stmt = ImportStatementUseCase { repo }
        .execute(ImportStatementInput {
            agency_id: ctx.agency.id,
            bank_name: dto.bank_name,
            account_number: dto.account_number,
            statement_date: dto.statement_date,
            opening_balance: dto.opening_balance,
            closing_balance: dto.closing_balance,
            lines: dto
                .lines
                .into_iter()
                .map(|l| ImportStatementLineInput2 {
                    value_date: l.value_date,
                    description: l.description,
                    amount: l.amount,
                    reference: l.reference,
                })
                .collect(),
            created_by: user.user_id,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(BankStatementResponse::from(stmt))))
}

/// POST /api/v1/bank-statements/:id/lines/:line_id/match
#[utoipa::path(
    post, path = "/api/v1/bank-statements/{id}/lines/{line_id}/match",
    params(
        ("id"      = Uuid, Path, description = "Statement UUID"),
        ("line_id" = Uuid, Path, description = "Statement line UUID"),
    ),
    request_body = MatchLineDto,
    responses(
        (status = 200, description = "Line matched"),
        (status = 404, description = "Not found",      body = ErrorResponse),
        (status = 409, description = "Already matched", body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "BankReconciliation", security(("bearer_token" = []))
)]
pub async fn match_line(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path((_id, line_id)): Path<(Uuid, Uuid)>,
    Json(dto): Json<MatchLineDto>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo: Arc<dyn BankReconciliationRepository> =
        Arc::new(PgBankReconciliationRepo::new(ctx.pool));
    MatchLineUseCase { repo }
        .execute(MatchLineInput {
            agency_id: ctx.agency.id,
            line_id,
            journal_entry_id: dto.journal_entry_id,
        })
        .await?;

    Ok(StatusCode::OK)
}

/// POST /api/v1/bank-statements/:id/lines/:line_id/unmatch
#[utoipa::path(
    post, path = "/api/v1/bank-statements/{id}/lines/{line_id}/unmatch",
    params(
        ("id"      = Uuid, Path, description = "Statement UUID"),
        ("line_id" = Uuid, Path, description = "Statement line UUID"),
    ),
    responses(
        (status = 200, description = "Match cleared"),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "BankReconciliation", security(("bearer_token" = []))
)]
pub async fn unmatch_line(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path((_id, line_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo: Arc<dyn BankReconciliationRepository> =
        Arc::new(PgBankReconciliationRepo::new(ctx.pool));
    UnmatchLineUseCase { repo }
        .execute(UnmatchLineInput {
            agency_id: ctx.agency.id,
            line_id,
        })
        .await?;

    Ok(StatusCode::OK)
}

/// GET /api/v1/bank-statements/:id/reconciliation
#[utoipa::path(
    get, path = "/api/v1/bank-statements/{id}/reconciliation",
    params(("id" = Uuid, Path, description = "Statement UUID")),
    responses(
        (status = 200, description = "Reconciliation report", body = ReconciliationReportResponse),
        (status = 404, description = "Not found",             body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "BankReconciliation", security(("bearer_token" = []))
)]
pub async fn get_reconciliation_report(
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

    let repo: Arc<dyn BankReconciliationRepository> =
        Arc::new(PgBankReconciliationRepo::new(ctx.pool));
    let report = GetReconciliationReportUseCase { repo }
        .execute(ctx.agency.id, id)
        .await?;

    Ok(Json(ReconciliationReportResponse::from(report)))
}
