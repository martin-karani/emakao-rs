use axum::{Json, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use garde::Validate;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, use_cases::{agency::provision::ProvisionAgencyInput, auth::register::{RegisterInput, RegisterUseCase}}},
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        http::{
            dto::{
                agency::CreateAgencyDto,
                openfga::{DeleteTupleDto, UpdateAuthModelDto, WriteTupleDto}, staff::CreateStaffUserDto,
            },
            responses::{agency::AgencyResponse, openfga::ModelVersionResponse, staff::CreatedStaffUserResponse},
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
/// **Platform-admin only** (`X-Admin-Key` header or JWT with
/// `role = platform_admin`).
///
/// Creates a fully-active staff user with a hashed password.  Use this to
/// provision the first `admin` for a newly created agency.  For subsequent
/// staff members, prefer the invite flow (`POST /api/v1/staff/invite`).
///
/// The user is immediately active and can log in at
/// `POST /api/v1/auth/login` with the supplied credentials.
/// Set `must_change_password: true` (coming soon) when you want the user to
/// rotate the password on first login.
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
 
    // Validate role before touching the database.
    if !["admin", "manager", "agent"].contains(&dto.role.as_str()) {
        return Err(AppError::Validation(
            "role must be one of: admin, manager, agent".into(),
        ));
    }
 
    // Guard: agency must exist and be active.
    state
        .identity
        .auth_repo
        .find_agency_by_id(agency_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("agency {agency_id} not found")))?;
 
    let user = RegisterUseCase::new(
        state.identity.auth_repo.clone(),
        state.identity.auth_port.clone(),
    )
    .execute(RegisterInput {
        agency_id,
        email: dto.email.trim().to_lowercase(),
        password: dto.password,
        role: dto.role.clone(),
    })
    .await?;
 
    tracing::info!(
        user_id   = %user.id,
        agency_id = %agency_id,
        role      = %dto.role,
        "platform admin seeded staff user"
    );
 
    Ok((
        StatusCode::CREATED,
        Json(CreatedStaffUserResponse {
            user_id: user.id,
            email: user.email.unwrap_or_default(),
            role: dto.role,
            is_active: true,
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
pub async fn write_permission_tuple(
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
pub async fn delete_permission_tuple(
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
pub async fn update_auth_model(
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
