// src/presentation/http/handlers/document.rs

use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        helpers::upload::{validate_mime, validate_size},
        use_cases::document::{
            list_documents::{
                DeleteDocumentUseCase, GetDocumentUseCase, ListDocumentsInput, ListDocumentsUseCase,
            },
            upload_document::{UploadDocumentInput, UploadDocumentUseCase},
        },
    },
    domain::{auth::AuthenticatedUser, document::DocumentType},
    infrastructure::db::document_repository_sqlx::PgDocumentRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::document::ListDocumentsParams, helpers::permission::check_permission,
            responses::document::DocumentResponse,
        },
    },
};

/// GET /api/v1/documents
#[utoipa::path(
    get, path = "/api/v1/documents",
    params(ListDocumentsParams),
    responses(
        (status = 200, description = "List of documents", body = Vec<DocumentResponse>),
        (status = 401, description = "Unauthorised",      body = ErrorResponse),
        (status = 403, description = "Forbidden",         body = ErrorResponse),
    ),
    tag = "Documents", security(("bearer_token" = []))
)]
pub async fn list_documents(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListDocumentsParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let doc_type = params
        .document_type
        .as_deref()
        .and_then(DocumentType::from_str);
    let repo = Arc::new(PgDocumentRepo::new(ctx.pool));
    let docs = ListDocumentsUseCase { repo }
        .execute(ListDocumentsInput {
            agency_id: ctx.agency.id,
            property_id: params.property_id,
            unit_id: params.unit_id,
            resident_id: params.resident_id,
            agreement_id: params.agreement_id,
            work_order_id: params.work_order_id,
            document_type: doc_type,
            limit: params.limit.min(100),
            offset: params.offset,
        })
        .await?;

    let responses = DocumentResponse::from_documents(docs, &*state.storage).await?;
    Ok(Json(responses))
}

/// GET /api/v1/documents/:id
#[utoipa::path(
    get, path = "/api/v1/documents/{id}",
    params(("id" = Uuid, Path, description = "Document UUID")),
    responses(
        (status = 200, description = "Document found", body = DocumentResponse),
        (status = 404, description = "Not found",      body = ErrorResponse),
        (status = 401, description = "Unauthorised",   body = ErrorResponse),
    ),
    tag = "Documents", security(("bearer_token" = []))
)]
pub async fn get_document(
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

    let repo = Arc::new(PgDocumentRepo::new(ctx.pool));
    let doc = GetDocumentUseCase { repo }
        .execute(ctx.agency.id, id)
        .await?;
    let response = DocumentResponse::from_document(doc, &*state.storage).await?;
    Ok(Json(response))
}

/// POST /api/v1/documents
#[utoipa::path(
    post, path = "/api/v1/documents",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Uploaded and persisted", body = DocumentResponse),
        (status = 400, description = "Missing file",           body = ErrorResponse),
        (status = 413, description = "File exceeds 20 MB",     body = ErrorResponse),
        (status = 422, description = "Validation error",        body = ErrorResponse),
        (status = 401, description = "Unauthorised",            body = ErrorResponse),
    ),
    tag = "Documents", security(("bearer_token" = []))
)]
pub async fn upload_document(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name = String::from("upload");
    let mut mime_type = String::from("application/octet-stream");
    let mut document_type_str = String::from("other");
    let mut title: Option<String> = None;
    let mut notes: Option<String> = None;
    let mut property_id: Option<Uuid> = None;
    let mut unit_id: Option<Uuid> = None;
    let mut resident_id: Option<Uuid> = None;
    let mut agreement_id: Option<Uuid> = None;
    let mut work_order_id: Option<Uuid> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("multipart error: {e}")))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                file_name = field.file_name().unwrap_or("upload").to_string();
                mime_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::Validation(format!("read error: {e}")))?;
                file_bytes = Some(bytes.to_vec());
            }
            "document_type" => {
                document_type_str = field
                    .text()
                    .await
                    .map_err(|e| AppError::Validation(e.to_string()))?;
            }
            "title" => title = Some(field.text().await.unwrap_or_default()),
            "notes" => notes = Some(field.text().await.unwrap_or_default()),
            "property_id" => property_id = parse_uuid_field(field.text().await.ok()),
            "unit_id" => unit_id = parse_uuid_field(field.text().await.ok()),
            "resident_id" => resident_id = parse_uuid_field(field.text().await.ok()),
            "agreement_id" => agreement_id = parse_uuid_field(field.text().await.ok()),
            "work_order_id" => work_order_id = parse_uuid_field(field.text().await.ok()),
            _ => {}
        }
    }

    let file_bytes =
        file_bytes.ok_or_else(|| AppError::Validation("missing `file` field".to_string()))?;

    validate_mime(&mime_type)?;
    validate_size(&file_bytes)?;

    let document_type = DocumentType::from_str(&document_type_str).unwrap_or(DocumentType::Other);

    let repo = Arc::new(PgDocumentRepo::new(ctx.pool));
    let doc = UploadDocumentUseCase {
        repo,
        storage: state.storage.clone(),
    }
    .execute(UploadDocumentInput {
        agency_id: ctx.agency.id,
        uploaded_by: user.user_id,
        file_name,
        mime_type,
        file_bytes,
        document_type,
        title,
        notes,
        property_id,
        unit_id,
        resident_id,
        agreement_id,
        work_order_id,
    })
    .await?;

    let response = DocumentResponse::from_document(doc, &*state.storage).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// DELETE /api/v1/documents/:id
#[utoipa::path(
    delete, path = "/api/v1/documents/{id}",
    params(("id" = Uuid, Path, description = "Document UUID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Documents", security(("bearer_token" = []))
)]
pub async fn delete_document(
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

    let repo = Arc::new(PgDocumentRepo::new(ctx.pool));
    DeleteDocumentUseCase {
        repo,
        storage: state.storage.clone(),
    }
    .execute(ctx.agency.id, id)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

fn parse_uuid_field(s: Option<String>) -> Option<Uuid> {
    s.and_then(|v| Uuid::parse_str(&v).ok())
}
