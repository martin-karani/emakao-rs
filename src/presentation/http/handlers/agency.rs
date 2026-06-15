//! HTTP handlers for agency operations:
//!
//! **Platform admin** (`/api/v1/admin/agencies/...`)
//! - Provision agencies, seed staff, manage OpenFGA tuples/models
//!
//! **Tenant customisation** (`/api/agency/...`)
//! - Settings, third-party integrations, portfolio billing summaries

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use garde::Validate;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{billing_repository::BillingRepository, role_repository::RoleRepository},
        use_cases::{
            agency::{
                deactivate_integration::{DeactivateIntegrationInput, DeactivateIntegrationUseCase},
                list_integrations::ListIntegrationsUseCase,
                provision::ProvisionAgencyInput,
                settings_update::{self, UpdateAgencySettingsCommand},
                upsert_integration::{UpsertIntegrationInput, UpsertIntegrationUseCase},
            },
            auth::register::{RegisterInput, RegisterUseCase},
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::{
        agency_repository_sqlx::PgAgencyRepo,
        billing_repository_sqlx::PgBillingRepo,
        pool::AgencyPool,
        role_repository_sqlx::PgRoleRepo,
    },
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        http::{
            dto::{
                agency::{CreateAgencyDto, PatchAgencySettingsDto, UpsertAgencyIntegrationDto},
                openfga::{DeleteTupleDto, UpdateAuthModelDto, WriteTupleDto},
                staff::CreateStaffUserDto,
            },
            responses::{
                agency::{
                    AgencyIntegrationResponse, AgencyResponse, AgencySettingsResponse,
                    PatchAgencySettingsResponse,
                },
                openfga::ModelVersionResponse,
                property::BillingSummaryResponse,
                staff::CreatedStaffUserResponse,
            },
        },
    },
};




/// POST /api/v1/admin/agencies
///
/// Creates an agency row, provisions the agency Postgres schema (migrations
/// included), creates an OpenFGA store, seeds the default authorization model,
/// and persists the `store_id`.  All in one atomic-ish flow.
#[utoipa::path(
    post,
    path = "/api/v1/admin/agencies",
    request_body = CreateAgencyDto,
    responses(
        (status = 201, description = "Agency created", body = AgencyResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Admin"
    
)]
pub async fn create_agency(
    State(state): State<AppState>,
    Json(dto): Json<CreateAgencyDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let agency = state
        .agency_uc
        .provision
        .execute(ProvisionAgencyInput {
            name: dto.name,
            slug: dto.slug,
            country_code: "KE".to_string(),
            currency_code: "KES".to_string(),
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(AgencyResponse {
            id: agency.id,
            name: agency.name,
            slug: agency.slug,
            schema_name: agency.schema_name,
            fga_store_id: agency.fga_store_id,
            status: agency.status,
        }),
    ))
}



/// POST /api/v1/admin/agencies/{agency_id}/staff
///
/// **System-admin only** (`X-Admin-Key` header or JWT with
/// `role = admin`).
///
/// Creates a fully-active staff user with a hashed password.  Use this to
/// provision the first `agency_owner` for a newly created agency.  For subsequent
/// staff members, prefer the invite flow (`POST /api/v1/staff/invite`).
///
/// The user is immediately active and can log in at
/// `POST /api/v1/auth/login` with the supplied credentials.
///
/// ## FGA side-effect
///
/// On success, an OpenFGA agency-membership tuple is written:
///
/// ```text
/// user:{user_id}  —  {role}  —  agency:{agency_id}
/// ```
///
/// This enables the full fine-grained permission chain (`property.can_view`,
/// `agreement.can_edit`, etc.) from the moment the account is created.
/// If the FGA write fails, the error is logged but the user is still created —
/// the tuple can be re-synced via
/// `POST /api/v1/admin/agencies/{fga_store_id}/permissions/tuples`.
#[utoipa::path(
    post,
    path = "/api/v1/admin/agencies/{agency_id}/staff",
    params(("agency_id" = Uuid, Path, description = "Target agency UUID")),
    request_body = CreateStaffUserDto,
    responses(
        (status = 201, description = "Staff user created",           body = CreatedStaffUserResponse),
        (status = 404, description = "Agency not found",             body = ErrorResponse),
        (status = 409, description = "Email already registered",     body = ErrorResponse),
        (status = 422, description = "Validation / role error",      body = ErrorResponse),
    ),
    tag = "Admin"
)]
pub async fn create_staff_user(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
    Json(dto): Json<CreateStaffUserDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    // Guard: agency must exist and be active.  Also capture fga_store_id so
    // we can write the FGA tuple without a second DB round-trip.
    let agency = state
        .identity
        .agency_repo
        .find_agency_by_id(agency_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("agency {agency_id} not found")))?;

    // Validate role exists in target agency
    let target_pool = state.infra.tenant_pools.for_agency(agency_id).await
        .map_err(|e| AppError::InternalServer(format!("failed to get pool for agency: {e}")))?;
    let role_repo = PgRoleRepo::new(target_pool);
    if role_repo.find_by_name(agency_id, &dto.role).await?.is_none() {
        return Err(AppError::Validation(format!("role '{}' does not exist in agency {}", dto.role, agency_id)));
    }

    let user = RegisterUseCase::new(
        state.identity.auth_repo.clone(),
        state.identity.agency_repo.clone(),
        state.identity.auth_port.clone(),
    )
    .execute(RegisterInput {
        agency_id,
        email: dto.email.trim().to_lowercase(),
        password: dto.password,
        role: dto.role.clone(),
    })
    .await?;

    // ── Write OpenFGA agency-membership tuple ──────────────────────────────
    //
    // This write is FATAL: if it fails the handler returns an error.
    // A staff user without an FGA tuple can log in but every permission
    // check will return FORBIDDEN — an unusable, confusing state.
    let store_id = agency.fga_store_id.as_deref().ok_or_else(|| {
        AppError::InternalServer(
            "Agency has no OpenFGA store configured — contact platform admin".into(),
        )
    })?;

    let fga_user   = format!("user:{}", user.id);
    let fga_object = format!("agency:{agency_id}");

    // Map custom roles to the three base FGA relations.
    let fga_relation = match dto.role.as_str() {
        "admin" | "agency_owner" => "agency_owner",
        "manager"                => "manager",
        _                        => "agent",
    };

    state
        .openfga
        .write_tuple(store_id, &fga_user, fga_relation, &fga_object)
        .await
        .map_err(|e| {
            tracing::error!(
                user_id   = %user.id,
                agency_id = %agency_id,
                role      = %dto.role,
                error     = %e,
                "FGA tuple write failed — aborting staff user creation",
            );
            AppError::InternalServer(format!("Failed to write FGA permission tuple: {e}"))
        })?;

    tracing::info!(
        user_id   = %user.id,
        agency_id = %agency_id,
        role      = %dto.role,
        fga_rel   = %fga_relation,
        store_id  = %store_id,
        "FGA agency-membership tuple written for seeded staff user",
    );

    tracing::info!(
        user_id   = %user.id,
        agency_id = %agency_id,
        role      = %dto.role,
        "system admin seeded staff user",
    );

    Ok((
        StatusCode::CREATED,
        Json(CreatedStaffUserResponse {
            user_id:              user.id,
            email:                user.email.unwrap_or_default(),
            role:                 dto.role,
            is_active:            true,
            must_change_password: false,
        }),
    ))
}


/// POST /api/v1/admin/agencies/:fga_store_id/permissions/tuples
///
/// Grant a relation tuple inside the agency's FGA store.
///
/// Example body:
/// ```json
/// { "user": "user:abc123", "relation": "manager", "object": "property:xyz789" }
/// ```
#[utoipa::path(
    post,
    path = "/api/v1/admin/agencies/{fga_store_id}/permissions/tuples",
    params(("fga_store_id" = String, Path, description = "OpenFGA store ID")),
    request_body = WriteTupleDto,
    responses(
        (status = 204, description = "Permission tuple written"),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Admin"
)]
pub async fn grant_permission_tuple(
    State(state): State<AppState>,
    axum::extract::Path(fga_store_id): axum::extract::Path<String>,
    Json(dto): Json<WriteTupleDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    state
        .openfga
        .write_tuple(&fga_store_id, &dto.user, &dto.relation, &dto.object)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/admin/agencies/:fga_store_id/permissions/tuples
///
/// Revoke a relation tuple from the agency's FGA store.
#[utoipa::path(
    delete,
    path = "/api/v1/admin/agencies/{fga_store_id}/permissions/tuples",
    params(("fga_store_id" = String, Path, description = "OpenFGA store ID")),
    request_body = DeleteTupleDto,
    responses(
        (status = 204, description = "Permission tuple deleted"),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
    ),
    tag = "Admin"
)]
pub async fn revoke_permission_tuple(
    State(state): State<AppState>,
    axum::extract::Path(fga_store_id): axum::extract::Path<String>,
    Json(dto): Json<DeleteTupleDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .openfga
        .delete_tuple(&fga_store_id, &dto.user, &dto.relation, &dto.object)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/admin/agencies/:fga_store_id/permissions/model
///
/// Publish a new authorization model version into the agency's FGA store.
///
/// Use this when an agency needs custom types or relations beyond the default
/// model.  Old model versions remain intact; only new `check` calls will
/// evaluate the new version.
#[utoipa::path(
    post,
    path = "/api/v1/admin/agencies/{fga_store_id}/permissions/model",
    params(("fga_store_id" = String, Path, description = "OpenFGA store ID")),
    request_body = UpdateAuthModelDto,
    responses(
        (status = 200, description = "Authorization model updated", body = ModelVersionResponse),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
    ),
    tag = "Admin"
)]
pub async fn publish_auth_model(
    State(state): State<AppState>,
    axum::extract::Path(fga_store_id): axum::extract::Path<String>,
    Json(dto): Json<UpdateAuthModelDto>,
) -> Result<impl IntoResponse, AppError> {
    let model_id = state
        .openfga
        .write_auth_model(&fga_store_id, &dto.model)
        .await?;

    Ok(Json(ModelVersionResponse {
        authorization_model_id: model_id,
    }))
}

// ── Tenant settings ───────────────────────────────────────────────────────────

/// GET /api/agency/settings
#[utoipa::path(
    get,
    path = "/api/agency/settings",
    tag = "Agency",
    responses(
        (status = 200, description = "Agency settings", body = AgencySettingsResponse),
        (status = 401, description = "Unauthorised"),
        (status = 500, description = "Internal error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_agency_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let settings = state
        .customisation()
        .settings
        .get_or_load(user.agency_id, state.infra.tenant_pools.platform())
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    Ok(Json(AgencySettingsResponse::from(settings.as_ref())))
}

/// PATCH /api/agency/settings
#[utoipa::path(
    patch,
    path = "/api/agency/settings",
    tag = "Agency",
    request_body = PatchAgencySettingsDto,
    responses(
        (status = 200, description = "Settings updated"),
        (status = 400, description = "Invalid patch (must be a JSON object)"),
        (status = 401, description = "Unauthorised"),
        (status = 403, description = "Forbidden — requires agency_settings:write"),
        (status = 500, description = "Internal error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn patch_agency_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(body): Json<PatchAgencySettingsDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    settings_update::execute(
        &Arc::new(state.clone()),
        UpdateAgencySettingsCommand {
            agency_id: user.agency_id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            ip_address: None,
            patch: body.patch,
        },
    )
    .await
    .map_err(|e| AppError::InternalServer(e.to_string()))?;

    Ok(Json(PatchAgencySettingsResponse {
        message: "Settings updated".into(),
    }))
}

// ── Tenant integrations ───────────────────────────────────────────────────────

/// GET /api/agency/integrations
pub async fn list_agency_integrations(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(AgencyPool(pool)): Extension<AgencyPool>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgAgencyRepo::new_with_encryption(
        pool,
        state.customisation().enc_key.clone(),
    ));

    let integrations = ListIntegrationsUseCase::new(repo)
        .execute(user.agency_id)
        .await?;

    Ok(Json(
        integrations
            .into_iter()
            .map(AgencyIntegrationResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// PUT /api/agency/integrations/{provider_type}/{provider_key}
pub async fn upsert_agency_integration(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(AgencyPool(pool)): Extension<AgencyPool>,
    Path((provider_type, provider_key)): Path<(String, String)>,
    Json(body): Json<UpsertAgencyIntegrationDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgAgencyRepo::new_with_encryption(
        pool.clone(),
        state.customisation().enc_key.clone(),
    ));

    UpsertIntegrationUseCase::new(repo)
        .execute(UpsertIntegrationInput {
            agency_id: user.agency_id,
            provider_type: provider_type.clone(),
            provider_key: provider_key.clone(),
            credentials: body.credentials,
            settings: body.settings,
        })
        .await?;

    let enc_key = &*state.customisation().enc_key;
    state
        .customisation()
        .providers
        .load_agency(user.agency_id, &pool, enc_key)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

    state
        .customisation()
        .audit
        .log(crate::infrastructure::audit::AuditEvent {
            agency_id: user.agency_id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: format!("integration.{provider_type}.{provider_key}.upserted"),
            entity_type: "agency_integration".into(),
            entity_id: user.agency_id,
            old_data: None,
            new_data: Some(
                serde_json::json!({ "provider_type": provider_type, "provider_key": provider_key }),
            ),
            ip_address: None,
        });

    Ok(StatusCode::OK)
}

/// DELETE /api/agency/integrations/{provider_type}/{provider_key}
pub async fn deactivate_agency_integration(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(AgencyPool(pool)): Extension<AgencyPool>,
    Path((provider_type, provider_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgAgencyRepo::new_with_encryption(
        pool,
        state.customisation().enc_key.clone(),
    ));

    DeactivateIntegrationUseCase::new(repo)
        .execute(DeactivateIntegrationInput {
            agency_id: user.agency_id,
            provider_type: provider_type.clone(),
            provider_key: provider_key.clone(),
        })
        .await?;

    match provider_type.as_str() {
        "sms" => {
            state.customisation().providers.sms.agency.remove(&user.agency_id);
        }
        "email" => {
            state.customisation().providers.email.remove(&user.agency_id);
        }
        "payment" => {
            state.customisation().providers.payment.remove(&user.agency_id);
        }
        "storage" => {
            state.customisation().providers.storage.remove(&user.agency_id);
        }
        _ => {}
    }

    state
        .customisation()
        .audit
        .log(crate::infrastructure::audit::AuditEvent {
            agency_id: user.agency_id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: format!("integration.{provider_type}.{provider_key}.deactivated"),
            entity_type: "agency_integration".into(),
            entity_id: user.agency_id,
            old_data: None,
            new_data: None,
            ip_address: None,
        });

    Ok(StatusCode::NO_CONTENT)
}

// ── Portfolio billing summary ─────────────────────────────────────────────────

/// GET /api/agency/billing-summary
pub async fn list_property_billing_summaries(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let repo = PgBillingRepo::for_agency(tenant_pool(&state, user.agency_id).await?, user.agency_id);
    let rows = repo.get_billing_summary_for_agency(user.agency_id).await?;

    let summaries: Vec<BillingSummaryResponse> = rows
        .into_iter()
        .map(|r| {
            let late_fee = if r.late_fee_value > rust_decimal::Decimal::ZERO {
                format!(
                    "{} {} ({}d grace)",
                    if r.late_fee_type == "flat" {
                        r.currency_code.clone()
                    } else {
                        String::new()
                    },
                    r.late_fee_value,
                    r.late_fee_grace_days
                )
            } else {
                "None".to_string()
            };

            let mut utils = Vec::new();
            if r.water_rate_per_unit > rust_decimal::Decimal::ZERO {
                utils.push(format!("Water: {}/unit", r.water_rate_per_unit));
            }
            if r.garbage_fee_kes > rust_decimal::Decimal::ZERO {
                utils.push(format!("Garbage: {}", r.garbage_fee_kes));
            }
            if r.security_fee_kes > rust_decimal::Decimal::ZERO {
                utils.push(format!("Security: {}", r.security_fee_kes));
            }

            BillingSummaryResponse {
                property_id: r.property_id,
                property_name: r.property_name,
                currency_code: r.currency_code,
                rent_due_day: r.rent_due_day,
                late_fee_summary: late_fee,
                utility_summary: if utils.is_empty() {
                    "None".to_string()
                } else {
                    utils.join(", ")
                },
            }
        })
        .collect();

    Ok(Json(summaries))
}

async fn tenant_pool(state: &AppState, agency_id: Uuid) -> Result<sqlx::PgPool, AppError> {
    state
        .infra
        .tenant_pools
        .for_agency(agency_id)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))
}
