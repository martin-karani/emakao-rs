use std::{collections::HashSet, sync::Arc};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::role::{
            create_role::{CreateRoleInput, CreateRoleUseCase},
            delete_role::DeleteRoleUseCase,
            list_roles::ListRolesUseCase,
            update_role::{UpdateRoleInput, UpdateRoleUseCase},
        },
    },
    domain::auth::AuthenticatedUser,
    domain::role::PermissionDefinition,
    infrastructure::db::role_repository_sqlx::PgRoleRepo,
    presentation::app_state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub permissions: HashSet<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub permissions: HashSet<String>,
}

#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: Uuid,
    pub name: String,
    pub permissions: HashSet<String>,
    pub is_system: bool,
}

pub async fn list_roles(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "roles:read")
        .await?;

    let repo = Arc::new(PgRoleRepo::new(state.infra.tenant_pools.platform_pool()));
    let roles = ListRolesUseCase::new(repo).execute(user.agency_id).await?;

    Ok(Json(roles))
}

pub async fn list_permissions(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "roles:read")
        .await?;

    let permissions = PermissionDefinition::all();
    Ok(Json(permissions))
}

pub async fn create_role(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(body): Json<CreateRoleRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "roles:write")
        .await?;

    let repo = Arc::new(PgRoleRepo::new(state.infra.tenant_pools.platform_pool()));
    let role = CreateRoleUseCase::new(repo)
        .execute(CreateRoleInput {
            agency_id: user.agency_id,
            name: body.name,
            permissions: body.permissions,
        })
        .await?;

    Ok(Json(role))
}

pub async fn update_role(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(role_id): Path<Uuid>,
    Json(body): Json<UpdateRoleRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "roles:write")
        .await?;

    let repo = Arc::new(PgRoleRepo::new(state.infra.tenant_pools.platform_pool()));
    let role = UpdateRoleUseCase::new(repo, state.customisation().permissions.clone())
        .execute(UpdateRoleInput {
            id: role_id,
            agency_id: user.agency_id,
            permissions: body.permissions,
        })
        .await?;

    Ok(Json(role))
}

pub async fn delete_role(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(role_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "roles:write")
        .await?;

    let repo = Arc::new(PgRoleRepo::new(state.infra.tenant_pools.platform_pool()));
    DeleteRoleUseCase::new(repo)
        .execute(user.agency_id, role_id)
        .await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
