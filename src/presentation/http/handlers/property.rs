// src/presentation/http/handlers/property.rs
//
// Refactored to use the customisation layer:
//   • require_below_limit! — enforces "max_properties" plan limit
//   • require_feature!     — gates "property_management" feature flag
//   • AuditLogger          — emits events on create / update / delete
//   • AgencySettings       — reads workflow defaults (country_code, currency)
//     via the Arc<AgencySettings> already in request extensions
//     (injected for free by resolve_agency_context middleware)
//
// Nothing else changes: same use-cases, same DTOs, same OpenFGA checks,
// same ResolvedSubscription extension, same AgencyContext extractor.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::property::{
            create_property::{CreatePropertyInput, CreatePropertyUseCase},
            delete_property::DeletePropertyUseCase,
            get_property::GetPropertyUseCase,
            list_properties::ListPropertiesUseCase,
            update_property::UpdatePropertyUseCase,
        },
    },
    domain::{
        agency_settings::AgencySettings, auth::AuthenticatedUser, property::UpdatePropertyCommand,
    },
    infrastructure::{audit::AuditEvent, db::property_repository_sqlx::PgPropertyRepo},
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::property::{CreatePropertyDto, ListPropertiesParams, UpdatePropertyDto},
            helpers::permission::check_permission,
            responses::property::PropertyResponse,
        },
        middleware::subscription::ResolvedSubscription,
        // Macros declared in src/presentation/macros.rs
        require_below_limit,
        require_feature,
    },
};

// ── List ──────────────────────────────────────────────────────────────────────

/// List all properties for the authenticated agency.
#[utoipa::path(
    get,
    path = "/api/v1/properties",
    params(ListPropertiesParams),
    responses(
        (status = 200, description = "List of properties",     body = Vec<PropertyResponse>),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 402, description = "Subscription inactive",  body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
pub async fn list_properties(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListPropertiesParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = ListPropertiesUseCase::new(repo);

    let items = usecase
        .execute(
            ctx.agency.id,
            params.property_type.map(|pt| format!("{:?}", pt)),
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(PropertyResponse::from)
            .collect::<Vec<_>>(),
    ))
}

// ── Get ───────────────────────────────────────────────────────────────────────

/// Get a single property by UUID.
#[utoipa::path(
    get,
    path = "/api/v1/properties/{id}",
    params(("id" = Uuid, Path, description = "Property UUID")),
    responses(
        (status = 200, description = "Property found",         body = PropertyResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Property not found",     body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
pub async fn get_property(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(&state, &ctx, &user, "can_view", &format!("property:{id}")).await?;

    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = GetPropertyUseCase::new(repo);
    let property = usecase.execute(ctx.agency.id, id).await?;

    Ok(Json(PropertyResponse::from(property)))
}

// ── Create ────────────────────────────────────────────────────────────────────

/// Create a new property.
#[utoipa::path(
    post,
    path = "/api/v1/properties",
    request_body = CreatePropertyDto,
    responses(
        (status = 201, description = "Property created",       body = PropertyResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 402, description = "Plan limit reached",     body = ErrorResponse),
        (status = 422, description = "Validation error",       body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
pub async fn create_property(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(_sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreatePropertyDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    // ── 1. Numeric plan limit ─────────────────────────────────────────────────
    // Count current properties; reject if at or above the plan's max_branches.
    let current_branches = state
        .subscription
        .repo
        .count_tenant_rows(ctx.agency.id, "properties")
        .await?;

    // Allow the first property (branch), but gate subsequent ones
    if current_branches > 0 {
        require_feature!(state, user.agency_id, "core_multi_branch");
    }
    require_below_limit!(state, user.agency_id, "max_branches", current_branches);

    // ── 3. Read AgencySettings for workflow defaults ───────────────────────────
    // Arc<AgencySettings> was inserted into extensions by resolve_agency_context.
    // Extracting it here is zero-cost (no DB / cache hit).
    let settings = ctx_settings(&state, user.agency_id).await;

    // Derive country_code from the agency locale, e.g. "en-KE" → "KE".
    let country_code = settings
        .as_ref()
        .and_then(|s| {
            s.extra
                .get("country_code")
                .and_then(|v| v.as_str().map(str::to_owned))
        })
        .unwrap_or_else(|| "KE".to_owned());

    // ── 4. Execute use-case ───────────────────────────────────────────────────
    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = CreatePropertyUseCase::new(repo);

    let property = usecase
        .execute(CreatePropertyInput {
            agency_id: ctx.agency.id,
            created_by: user.user_id,
            name: dto.name,
            address: dto.address,
            city: dto.city,
            property_type: dto.property_type,
            config: dto.config,
            work_order_prefix: dto.work_order_prefix,
        })
        .await?;

    // ── 5. Audit ──────────────────────────────────────────────────────────────
    if let Some(custom) = &state.custom {
        custom.audit.log(AuditEvent {
            agency_id: ctx.agency.id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: "property.created".to_string(),
            entity_type: "property".to_string(),
            entity_id: property.id,
            old_data: None,
            new_data: Some(serde_json::json!({
                "name":         property.name,
                "address":      property.address,
                "city":         property.city,
                "country_code": country_code,
                "property_type": format!("{:?}", property.property_type),
            })),
            ip_address: None,
        });
    }

    Ok((StatusCode::CREATED, Json(PropertyResponse::from(property))))
}

// ── Update ────────────────────────────────────────────────────────────────────

/// Update a property's mutable fields.
#[utoipa::path(
    put,
    path = "/api/v1/properties/{id}",
    params(("id" = Uuid, Path, description = "Property UUID")),
    request_body = UpdatePropertyDto,
    responses(
        (status = 200, description = "Property updated",       body = PropertyResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Property not found",     body = ErrorResponse),
        (status = 422, description = "Validation error",       body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
pub async fn update_property(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdatePropertyDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    check_permission(&state, &ctx, &user, "can_edit", &format!("property:{id}")).await?;

    // Capture the before-state for a field-level audit diff.
    let before = if let Some(custom) = &state.custom {
        if custom.audit.should_log_field_changes(ctx.agency.id).await {
            let repo = Arc::new(PgPropertyRepo::from(ctx.pool.clone()));
            let usecase = GetPropertyUseCase::new(repo);
            usecase.execute(ctx.agency.id, id).await.ok()
        } else {
            None
        }
    } else {
        None
    };

    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = UpdatePropertyUseCase::new(repo);

    let property = usecase
        .execute(UpdatePropertyCommand {
            id,
            agency_id: ctx.agency.id,
            name: dto.name,
            address: dto.address,
            city: dto.city,
        })
        .await?;

    // ── Audit ─────────────────────────────────────────────────────────────────
    if let Some(custom) = &state.custom {
        custom.audit.log(AuditEvent {
            agency_id: ctx.agency.id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: "property.updated".to_string(),
            entity_type: "property".to_string(),
            entity_id: property.id,
            old_data: before.as_ref().map(|p| {
                serde_json::json!({
                    "name":    p.name,
                    "address": p.address,
                    "city":    p.city,
                })
            }),
            new_data: Some(serde_json::json!({
                "name":    property.name,
                "address": property.address,
                "city":    property.city,
            })),
            ip_address: None,
        });
    }

    Ok(Json(PropertyResponse::from(property)))
}

// ── Delete ────────────────────────────────────────────────────────────────────

/// Delete a property (hard delete — irreversible).
#[utoipa::path(
    delete,
    path = "/api/v1/properties/{id}",
    params(("id" = Uuid, Path, description = "Property UUID")),
    responses(
        (status = 204, description = "Property deleted"),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Property not found",     body = ErrorResponse),
    ),
    tag = "Properties",
    security(("bearer_token" = []))
)]
pub async fn delete_property(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(&state, &ctx, &user, "can_edit", &format!("property:{id}")).await?;

    let repo = Arc::new(PgPropertyRepo::from(ctx.pool));
    let usecase = DeletePropertyUseCase::new(repo);
    usecase.execute(ctx.agency.id, id).await?;

    // ── Audit ─────────────────────────────────────────────────────────────────
    if let Some(custom) = &state.custom {
        custom.audit.log(AuditEvent {
            agency_id: ctx.agency.id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: "property.deleted".to_string(),
            entity_type: "property".to_string(),
            entity_id: id,
            old_data: None,
            new_data: None,
            ip_address: None,
        });
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── Internal helper ───────────────────────────────────────────────────────────

/// Load AgencySettings from the customisation cache.
/// Returns `None` gracefully when the customisation layer has not been
/// initialised (e.g. in tests) so callers can fall back to defaults.
async fn ctx_settings(state: &AppState, agency_id: Uuid) -> Option<Arc<AgencySettings>> {
    let custom = state.custom.as_ref()?;
    custom
        .settings
        .get_or_load(agency_id, state.infra.tenant_pools.platform())
        .await
        .map_err(|e| {
            tracing::warn!(agency = %agency_id, error = %e, "property handler: failed to load AgencySettings");
        })
        .ok()
}
