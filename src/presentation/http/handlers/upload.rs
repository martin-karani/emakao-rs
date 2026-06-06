//
// Generic multipart file upload endpoint for **ephemeral attachments**.
//
// Route:  POST /api/v1/upload
//
// ## When to use this endpoint
//
// Use this route for files that are *referenced* from another entity but not
// catalogued in the `documents` table:
//
//   - `payment_proof`  — proof-of-payment image attached to a payment claim
//   - `work_order`     — photo attached to a work order or comment
//   - `avatar`         — user profile picture
//
// For **persisted, tenant-owned documents** (lease agreements, NOCs, IDs, …)
// use `POST /api/v1/documents` instead.  That endpoint writes both an S3 object
// *and* a row to the `documents` table, and is the canonical source of truth
// for all searchable, auditable files.
//
// ## Request (multipart/form-data)
//
//   file       – binary file field (required)
//   context    – one of: payment_proof | work_order | avatar  (required)
//   entity_id  – UUID of the owning entity (optional; included in the S3 key)
//
// ## Response (201)
//
//   { "key": "agencies/…/payment_proof/…/uuid.jpg",
//     "url": "https://…",          ← 15-minute presigned GET URL
//     "content_type": "image/jpeg",
//     "size_bytes": 204800 }

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        helpers::upload::{attachment_s3_key, ext_from_mime, validate_mime, validate_size},
    },
    domain::auth::AuthenticatedUser,
    presentation::app_state::AppState,
};

// ── Allowed contexts ──────────────────────────────────────────────────────────

/// Contexts accepted by the generic upload endpoint.
///
/// `document` is intentionally absent — persisted files go through
/// `POST /api/v1/documents`.
const ALLOWED_CONTEXTS: &[&str] = &["payment_proof", "work_order", "avatar", "property_photo"];

// ── Response ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    /// Canonical S3 object key — store this in the referencing table
    /// (e.g. `payment_claims.proof_key`).
    pub key: String,
    /// 15-minute presigned GET URL for immediate client download.
    pub url: String,
    pub content_type: String,
    pub size_bytes: usize,
}

// ── Handler ───────────────────────────────────────────────────────────────────

/// POST /api/v1/upload
#[utoipa::path(
    post,
    path = "/api/v1/upload",
    request_body(
        content_type = "multipart/form-data",
        description  = "file (required), context (required), entity_id (optional UUID)"
    ),
    responses(
        (status = 201, description = "File uploaded",                   body = UploadResponse),
        (status = 400, description = "Missing file or invalid context", body = crate::presentation::error::ErrorResponse),
        (status = 413, description = "File exceeds 20 MB limit",        body = crate::presentation::error::ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Upload",
    security(("bearer_token" = []))
)]
pub async fn upload_file(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    // ── Parse multipart fields ────────────────────────────────────────────────
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;
    let mut context = String::new();
    let mut entity_id: Option<Uuid> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("multipart error: {e}")))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                content_type = field.content_type().map(str::to_owned);
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::Validation(format!("read error: {e}")))?;
                file_bytes = Some(bytes.to_vec());
            }

            "context" => {
                context = field
                    .text()
                    .await
                    .map_err(|e| AppError::Validation(format!("context field error: {e}")))?;

                if !ALLOWED_CONTEXTS.contains(&context.as_str()) {
                    return Err(AppError::Validation(format!(
                        "context must be one of: {}. \
                         For persisted documents use POST /api/v1/documents.",
                        ALLOWED_CONTEXTS.join(", ")
                    )));
                }
            }

            "entity_id" => {
                let raw = field
                    .text()
                    .await
                    .map_err(|e| AppError::Validation(format!("entity_id field error: {e}")))?;
                entity_id =
                    Some(raw.parse::<Uuid>().map_err(|_| {
                        AppError::Validation("entity_id must be a valid UUID".into())
                    })?);
            }

            _ => {} // ignore unknown fields
        }
    }

    // ── Require file field ────────────────────────────────────────────────────
    let bytes = file_bytes
        .ok_or_else(|| AppError::Validation("missing 'file' field in multipart body".into()))?;

    if context.is_empty() {
        return Err(AppError::Validation(format!(
            "missing 'context' field. Must be one of: {}",
            ALLOWED_CONTEXTS.join(", ")
        )));
    }

    // ── Validate MIME type and size (shared policy) ───────────────────────────
    let ct = content_type.unwrap_or_else(|| "application/octet-stream".into());
    validate_mime(&ct)?;
    validate_size(&bytes)?;

    // ── Build canonical S3 key ────────────────────────────────────────────────
    let agency_id = user.agency_id;
    let ext = ext_from_mime(&ct);
    let key = attachment_s3_key(agency_id, &context, entity_id, ext);

    // ── Upload via shared StoragePort ─────────────────────────────────────────
    let size_bytes = bytes.len();

    state
        .storage
        .upload(&key, bytes, &ct)
        .await
        .map_err(|e| AppError::ExternalService(format!("storage: {e}")))?;

    // Build presigned URL for immediate client use.
    let url = state
        .storage
        .get_url(&key)
        .await
        .map_err(|e| AppError::ExternalService(format!("presign: {e}")))?;

    tracing::info!(
        key        = %key,
        context    = %context,
        size_bytes = size_bytes,
        user_id    = %user.user_id,
        "attachment uploaded"
    );

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            key: key.to_string(),
            url,
            content_type: ct,
            size_bytes,
        }),
    ))
}
