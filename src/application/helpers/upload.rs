//
// Canonical shared helpers for every file-upload flow in the application.
//
// Both flows import from here:
//   1. Generic attachment upload  (POST /api/v1/upload)
//      — ephemeral files: payment proofs, work-order photos, avatars.
//      — stored in S3, key returned to caller; NOT written to `documents` table.
//
//   2. Persisted document upload  (POST /api/v1/documents)
//      — tenant-owned, catalogued files written to the `documents` table.
//
// Putting policy constants and key-building in one place means:
//   • size limit, MIME allow-list, and key scheme change in exactly one file.
//   • no silent drift between the two upload paths.

use uuid::Uuid;

use crate::application::errors::AppError;

// ── Policy constants ──────────────────────────────────────────────────────────

/// Maximum accepted file size for every upload endpoint.
pub const MAX_FILE_BYTES: usize = 20 * 1024 * 1024; // 20 MB

/// MIME types accepted by every upload endpoint.
/// Add new types here to enable them everywhere at once.
pub const ALLOWED_MIME_TYPES: &[&str] =
    &["image/jpeg", "image/png", "image/webp", "application/pdf"];

// ── Validation ────────────────────────────────────────────────────────────────

/// Returns `Err(Validation)` when `ct` is not in [`ALLOWED_MIME_TYPES`].
pub fn validate_mime(ct: &str) -> Result<(), AppError> {
    if !ALLOWED_MIME_TYPES.contains(&ct) {
        return Err(AppError::Validation(format!(
            "unsupported content type '{ct}'. Allowed: {}",
            ALLOWED_MIME_TYPES.join(", ")
        )));
    }
    Ok(())
}

/// Returns `Err(Validation)` when `bytes` exceeds [`MAX_FILE_BYTES`].
pub fn validate_size(bytes: &[u8]) -> Result<(), AppError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(AppError::Validation(format!(
            "file exceeds the maximum size of {} MB",
            MAX_FILE_BYTES / (1024 * 1024)
        )));
    }
    Ok(())
}

// ── Extension helper ──────────────────────────────────────────────────────────

/// Derives a canonical file extension from a validated MIME type.
///
/// Always use MIME-derived extensions (not the client-supplied filename) so the
/// stored extension is trustworthy.
pub fn ext_from_mime(ct: &str) -> &'static str {
    match ct {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "application/pdf" => "pdf",
        _ => "bin",
    }
}

// ── S3 key generation — single source of truth ────────────────────────────────
//
// All keys follow the pattern  `agencies/{agency_id}/...`  so every file is
// scoped to a tenant from the path alone. Never use the old bare
// `{agency_id}/{context}/...` pattern.

/// Canonical key for a **persisted document** (written to the `documents` table).
///
/// ```
/// agencies/{agency_id}/documents/{file_uuid}.{ext}
/// ```
pub fn document_s3_key(agency_id: Uuid, ext: &str) -> String {
    format!(
        "agencies/{}/documents/{}.{}",
        agency_id,
        Uuid::new_v4(),
        ext
    )
}

/// Canonical key for an **ephemeral attachment** (payment proof, work-order
/// image, avatar).  Not written to `documents`; the caller stores the key where
/// it belongs (e.g. `payment_claims.proof_key`).
///
/// ```
/// agencies/{agency_id}/{context}/{entity_id}/{file_uuid}.{ext}  (with entity)
/// agencies/{agency_id}/{context}/{file_uuid}.{ext}              (no entity)
/// ```
pub fn attachment_s3_key(
    agency_id: Uuid,
    context: &str,
    entity_id: Option<Uuid>,
    ext: &str,
) -> String {
    let file_uuid = Uuid::new_v4();
    match entity_id {
        Some(eid) => format!(
            "agencies/{}/{}/{}/{}.{}",
            agency_id, context, eid, file_uuid, ext
        ),
        None => format!("agencies/{}/{}/{}.{}", agency_id, context, file_uuid, ext),
    }
}
