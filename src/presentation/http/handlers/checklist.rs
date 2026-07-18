use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
        use_cases::checklist::{
            attach_to_property::AttachChecklistToPropertyInput,
            attach_to_property::AttachChecklistToPropertyUseCase,
            create_checklist::CreateChecklistInput, create_checklist::CreateChecklistUseCase,
            create_item::CreateChecklistItemInput, create_item::CreateChecklistItemUseCase,
            create_section::CreateChecklistSectionInput,
            create_section::CreateChecklistSectionUseCase, delete_checklist::DeleteChecklistInput,
            delete_checklist::DeleteChecklistUseCase,
            detach_from_property::DetachChecklistFromPropertyInput,
            detach_from_property::DetachChecklistFromPropertyUseCase,
            get_checklist::GetChecklistInput, get_checklist::GetChecklistUseCase,
            instantiate_tree::{
                InstantiateChecklistTreeInput, InstantiateChecklistTreeUseCase,
                InstantiateItemInput, InstantiateSectionInput,
            },
            list_checklists::ListChecklistsInput, list_checklists::ListChecklistsUseCase,
            list_items::ListChecklistItemsInput, list_items::ListChecklistItemsUseCase,
            list_property_checklists::ListPropertyChecklistsInput,
            list_property_checklists::ListPropertyChecklistsUseCase,
            list_sections::ListChecklistSectionsInput, list_sections::ListChecklistSectionsUseCase,
            update_checklist::UpdateChecklistInput, update_checklist::UpdateChecklistUseCase,
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::checklist_repository_sqlx::PgChecklistRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        http::{
            dto::checklist::{
                AttachChecklistToPropertyDto, CreateChecklistDto, CreateChecklistItemDto,
                CreateChecklistSectionDto, InstantiateChecklistTreeDto, UpdateChecklistDto,
            },
            responses::checklist::{
                ChecklistItemResponse, ChecklistResponse, ChecklistSectionResponse,
            },
        },
    },
};

async fn tenant_pool(state: &AppState, agency_id: Uuid) -> Result<sqlx::PgPool, AppError> {
    state
        .infra
        .tenant_pools
        .for_agency(agency_id)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))
}

#[utoipa::path(
    post,
    path = "/api/v1/checklists",
    request_body = CreateChecklistDto,
    responses(
        (status = 201, description = "Checklist created successfully", body = ChecklistResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn create_checklist(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateChecklistDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = CreateChecklistUseCase::new(repo);

    let checklist = usecase
        .execute(CreateChecklistInput {
            name: dto.name,
            description: dto.description,
            is_default: dto.is_default,
        })
        .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(ChecklistResponse::from(checklist)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/checklists/{id}",
    params(
        ("id" = Uuid, Path, description = "Checklist ID"),
    ),
    responses(
        (status = 200, description = "Checklist retrieved successfully", body = ChecklistResponse),
        (status = 404, description = "Checklist not found", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn get_checklist(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = GetChecklistUseCase::new(repo);

    let checklist = usecase
        .execute(GetChecklistInput { id })
        .await?;
    Ok(Json(ChecklistResponse::from(checklist)))
}

#[utoipa::path(
    get,
    path = "/api/v1/checklists",
    responses(
        (status = 200, description = "List of checklists retrieved successfully", body = [ChecklistResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn list_checklists(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = ListChecklistsUseCase::new(repo);

    let checklists = usecase.execute(ListChecklistsInput {}).await?;
    let response_list: Vec<ChecklistResponse> = checklists.into_iter().map(|c| c.into()).collect();
    Ok(Json(response_list))
}

#[utoipa::path(
    put,
    path = "/api/v1/checklists/{id}",
    params(
        ("id" = Uuid, Path, description = "Checklist ID"),
    ),
    request_body = UpdateChecklistDto,
    responses(
        (status = 200, description = "Checklist updated successfully", body = ChecklistResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Checklist not found", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn update_checklist(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateChecklistDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = UpdateChecklistUseCase::new(repo);

    let checklist = usecase
        .execute(UpdateChecklistInput {
            id,
            name: dto.name,
            description: dto.description,
            is_default: dto.is_default,
        })
        .await?;

    Ok(Json(ChecklistResponse::from(checklist)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/checklists/{id}",
    params(
        ("id" = Uuid, Path, description = "Checklist ID"),
    ),
    responses(
        (status = 204, description = "Checklist deleted successfully"),
        (status = 404, description = "Checklist not found", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn delete_checklist(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = DeleteChecklistUseCase::new(repo);

    usecase.execute(DeleteChecklistInput { id }).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/checklists/{checklist_id}/sections",
    params(
        ("checklist_id" = Uuid, Path, description = "Checklist ID"),
    ),
    request_body = CreateChecklistSectionDto,
    responses(
        (status = 201, description = "Section created successfully", body = ChecklistSectionResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn create_checklist_section(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(checklist_id): Path<Uuid>,
    Json(dto): Json<CreateChecklistSectionDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = CreateChecklistSectionUseCase::new(repo);

    let section = usecase
        .execute(CreateChecklistSectionInput {
            checklist_id,
            name: dto.name,
            description: dto.description,
            sort_order: dto.sort_order,
        })
        .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(ChecklistSectionResponse::from(section)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/checklists/{checklist_id}/sections",
    params(
        ("checklist_id" = Uuid, Path, description = "Checklist ID"),
    ),
    responses(
        (status = 200, description = "List of sections retrieved successfully", body = [ChecklistSectionResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn list_checklist_sections(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(checklist_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = ListChecklistSectionsUseCase::new(repo);

    let sections = usecase
        .execute(ListChecklistSectionsInput { checklist_id })
        .await?;
    let response_list: Vec<ChecklistSectionResponse> =
        sections.into_iter().map(|s| s.into()).collect();
    Ok(Json(response_list))
}

#[utoipa::path(
    post,
    path = "/api/v1/checklists/sections/{section_id}/items",
    params(
        ("section_id" = Uuid, Path, description = "Section ID"),
    ),
    request_body = CreateChecklistItemDto,
    responses(
        (status = 201, description = "Item created successfully", body = ChecklistItemResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn create_checklist_item(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(section_id): Path<Uuid>,
    Json(dto): Json<CreateChecklistItemDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = CreateChecklistItemUseCase::new(repo);

    let item = usecase
        .execute(CreateChecklistItemInput {
            section_id,
            name: dto.name,
            description: dto.description,
            checklist_type: dto.checklist_type,
            sort_order: dto.sort_order,
        })
        .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(ChecklistItemResponse::from(item)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/checklists/sections/{section_id}/items",
    params(
        ("section_id" = Uuid, Path, description = "Section ID"),
    ),
    responses(
        (status = 200, description = "List of items retrieved successfully", body = [ChecklistItemResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn list_checklist_items(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(section_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = ListChecklistItemsUseCase::new(repo);

    let items = usecase
        .execute(ListChecklistItemsInput { section_id })
        .await?;
    let response_list: Vec<ChecklistItemResponse> = items.into_iter().map(|i| i.into()).collect();
    Ok(Json(response_list))
}

#[utoipa::path(
    post,
    path = "/api/v1/properties/{property_id}/checklists",
    params(
        ("property_id" = Uuid, Path, description = "Property ID"),
    ),
    request_body = AttachChecklistToPropertyDto,
    responses(
        (status = 201, description = "Checklist attached successfully"),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn attach_checklist_to_property(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(property_id): Path<Uuid>,
    Json(dto): Json<AttachChecklistToPropertyDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = AttachChecklistToPropertyUseCase::new(repo);

    usecase
        .execute(AttachChecklistToPropertyInput {
            property_id,
            checklist_id: dto.checklist_id,
        })
        .await?;

    Ok(axum::http::StatusCode::CREATED)
}

#[utoipa::path(
    get,
    path = "/api/v1/properties/{property_id}/checklists",
    params(
        ("property_id" = Uuid, Path, description = "Property ID"),
    ),
    responses(
        (status = 200, description = "List of checklists retrieved successfully", body = [ChecklistResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn list_property_checklists(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(property_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = ListPropertyChecklistsUseCase::new(repo);

    let checklists = usecase
        .execute(ListPropertyChecklistsInput { property_id })
        .await?;
    let response_list: Vec<ChecklistResponse> = checklists.into_iter().map(|c| c.into()).collect();
    Ok(Json(response_list))
}

#[utoipa::path(
    delete,
    path = "/api/v1/properties/{property_id}/checklists/{checklist_id}",
    params(
        ("property_id" = Uuid, Path, description = "Property ID"),
        ("checklist_id" = Uuid, Path, description = "Checklist ID"),
    ),
    responses(
        (status = 204, description = "Checklist detached successfully"),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn detach_checklist_from_property(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((property_id, checklist_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = DetachChecklistFromPropertyUseCase::new(repo);

    usecase
        .execute(DetachChecklistFromPropertyInput {
            property_id,
            checklist_id,
        })
        .await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/properties/{property_id}/checklists/instantiate",
    params(
        ("property_id" = Uuid, Path, description = "Property ID"),
    ),
    request_body = InstantiateChecklistTreeDto,
    responses(
        (status = 201, description = "Checklist instantiated successfully", body = ChecklistResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Checklists",
    security(("bearer_token" = []))
)]
pub async fn instantiate_checklist_tree(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(property_id): Path<Uuid>,
    Json(dto): Json<InstantiateChecklistTreeDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let pool = tenant_pool(&state, user.agency_id).await?;
    let repo: Arc<dyn ChecklistRepository> = Arc::new(PgChecklistRepo::from(pool));
    let usecase = InstantiateChecklistTreeUseCase::new(repo);

    let checklist = usecase
        .execute(InstantiateChecklistTreeInput {
            property_id,
            template_id: dto.template_id,
            name: dto.name,
            description: dto.description,
            sections: dto
                .sections
                .into_iter()
                .map(|s| InstantiateSectionInput {
                    name: s.name,
                    description: s.description,
                    sort_order: s.sort_order,
                    items: s
                        .items
                        .into_iter()
                        .map(|i| InstantiateItemInput {
                            name: i.name,
                            description: i.description,
                            checklist_type: i.checklist_type,
                            sort_order: i.sort_order,
                        })
                        .collect(),
                })
                .collect(),
        })
        .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(ChecklistResponse::from(checklist)),
    ))
}
