//! HTTP handlers for staff management.
//!
//! ## Route ownership
//!
//! | Handler              | Method | Path                                          | Auth guard     |
//! |----------------------|--------|-----------------------------------------------|----------------|
//! | `invite_staff`       | POST   | `/api/v1/staff/invite`                        | staff JWT      |
//! | `list_staff`         | GET    | `/api/v1/staff`                               | staff JWT      |
//! | `get_staff_member`   | GET    | `/api/v1/staff/{membership_id}`               | staff JWT      |
//! | `deactivate_staff`   | DELETE | `/api/v1/staff/{membership_id}`               | staff JWT      |
//!
//! The platform-admin creation endpoint lives in `handlers/agency.rs`:
//! | `create_staff_user`  | POST   | `/api/v1/admin/agencies/{agency_id}/staff`    | admin key/JWT  |
//!
//! ## FGA tuple lifecycle
//!
//! Every staff membership has a corresponding OpenFGA tuple:
//!   `user:{user_id}` — `{role}` — `agency:{agency_id}`
//!
//! - **invite_staff**     → tuple is written inside `InviteStaffUseCase`.
//! - **deactivate_staff** → tuple is deleted here, immediately after the DB
//!   membership is deactivated.  Deletion is best-effort: a failure is logged
//!   but the handler still returns 204 because the user's JWT is already
//!   invalidated at the DB layer (membership `is_active = false`).

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
        use_cases::auth::invite_staff::{InviteStaffInput, InviteStaffUseCase},
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::role_repository_sqlx::PgRoleRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::staff::{InviteStaffDto, ListStaffParams},
            responses::staff::{InviteStaffResponse, StaffUserResponse},
        },
    },
};

// ── POST /api/v1/staff/invite ─────────────────────────────────────────────────

/// Invite a new staff member.
///
/// Creates an *inactive* platform user, binds them to the calling user's agency
/// with the requested role, writes the corresponding OpenFGA tuple so that
/// fine-grained permission checks are ready the moment the invite is accepted,
/// and sends a 48-hour invite email.  The invitee activates their account via
/// `POST /api/v1/auth/accept-invite`.
///
/// **Requires** a staff JWT (`admin`, `manager`, or `agent` role).
#[utoipa::path(
    post,
    path = "/api/v1/staff/invite",
    request_body = InviteStaffDto,
    responses(
        (status = 201, description = "Staff member invited",          body = InviteStaffResponse),
        (status = 409, description = "Email already registered",      body = ErrorResponse),
        (status = 422, description = "Validation error",              body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT",        body = ErrorResponse),
    ),
    tag = "Staff",
    security(("bearer_token" = []))
)]
pub async fn invite_staff(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(caller): Extension<AuthenticatedUser>,
    Json(dto): Json<InviteStaffDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    // Build the inviter display name from the JWT (user_id only).
    // In a richer setup you'd load the user's profile; this keeps it simple.
    let inviter_name = format!("Staff ({})", caller.user_id);

    let uc = InviteStaffUseCase {
        auth_repo: state.identity.auth_repo.clone(),
        role_repo: Arc::new(PgRoleRepo::new(state.pool())),
        auth_port: state.identity.auth_port.clone(),
        notifications: state.notifications.clone(),
        openfga: state.openfga.clone(),
    };

    let output = uc
        .execute(InviteStaffInput {
            agency_id: ctx.agency.id,
            fga_store_id: ctx.agency.fga_store_id.clone(),
            email: dto.email,
            first_name: dto.first_name,
            last_name: dto.last_name,
            role: dto.role,
            inviter_name,
            agency_name: Some(ctx.agency.name.clone()),
            portal_base_url: "https://app.emakao.co.ke".to_string(),
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(InviteStaffResponse {
            user_id: output.user_id,
            email: output.email,
            role: output.role,
            is_active: false,
            // Expose invite URL only in non-production environments.
            // In production, omit by returning None here.
            invite_url: Some(output.invite_url),
        }),
    ))
}

// ── GET /api/v1/staff ─────────────────────────────────────────────────────────

/// List all staff members for the agency.
///
/// Returns `admin`, `manager`, and `agent` role members, ordered by creation
/// date descending.  Includes pending (not-yet-activated) invites.
#[utoipa::path(
    get,
    path = "/api/v1/staff",
    params(ListStaffParams),
    responses(
        (status = 200, description = "List of staff members",    body = Vec<StaffUserResponse>),
        (status = 401, description = "Missing or invalid JWT",   body = ErrorResponse),
    ),
    tag = "Staff",
    security(("bearer_token" = []))
)]
pub async fn list_staff(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Query(params): Query<ListStaffParams>,
) -> Result<impl IntoResponse, AppError> {
    let members = state
        .identity
        .auth_repo
        .list_staff(
            ctx.agency.id,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        members
            .into_iter()
            .map(StaffUserResponse::from)
            .collect::<Vec<_>>(),
    ))
}

// ── GET /api/v1/staff/{membership_id} ────────────────────────────────────────

/// Get a single staff member by their membership ID.
///
/// `{membership_id}` is the `user_agency_roles.id` value returned by
/// `list_staff` and `invite_staff`.
#[utoipa::path(
    get,
    path = "/api/v1/staff/{membership_id}",
    params(("membership_id" = Uuid, Path, description = "user_agency_roles.id")),
    responses(
        (status = 200, description = "Staff member details",     body = StaffUserResponse),
        (status = 404, description = "Not found",                body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT",   body = ErrorResponse),
    ),
    tag = "Staff",
    security(("bearer_token" = []))
)]
pub async fn get_staff_member(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(membership_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let member = state
        .identity
        .auth_repo
        .find_staff_member(ctx.agency.id, membership_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("staff member {membership_id} not found")))?;

    Ok(Json(StaffUserResponse::from(member)))
}

// ── DELETE /api/v1/staff/{membership_id} ─────────────────────────────────────

/// Deactivate a staff membership.
///
/// Sets `user_agency_roles.is_active = false`.  The user can no longer log in,
/// but the record is retained for audit purposes.  This endpoint cannot be used
/// to deactivate yourself — callers attempting to deactivate their own
/// membership receive a `409 Conflict`.
///
/// After the DB deactivation succeeds the handler also revokes the OpenFGA
/// tuple (`user:{id}` — `{role}` — `agency:{id}`).  Tuple deletion is
/// best-effort: even if it fails, the member's JWT is invalidated at the DB
/// layer, so there is no immediate security exposure.  The stale tuple is safe
/// to leave in place until it is cleaned up via the admin permissions API.
#[utoipa::path(
    delete,
    path = "/api/v1/staff/{membership_id}",
    params(("membership_id" = Uuid, Path, description = "user_agency_roles.id")),
    responses(
        (status = 204, description = "Staff member deactivated"),
        (status = 404, description = "Not found",                    body = ErrorResponse),
        (status = 409, description = "Cannot deactivate yourself",   body = ErrorResponse),
        (status = 401, description = "Missing or invalid JWT",       body = ErrorResponse),
    ),
    tag = "Staff",
    security(("bearer_token" = []))
)]
pub async fn deactivate_staff(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(caller): Extension<AuthenticatedUser>,
    Path(membership_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // Fetch the member before deactivating so we have their user_id and role
    // for FGA tuple revocation and the self-deactivation guard.
    let member = state
        .identity
        .auth_repo
        .find_staff_member(ctx.agency.id, membership_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("staff member {membership_id} not found")))?;

    if member.user_id == caller.user_id {
        return Err(AppError::Conflict(
            "You cannot deactivate your own staff membership.".into(),
        ));
    }

    // ── Deactivate in the database ─────────────────────────────────────────
    state
        .identity
        .auth_repo
        .deactivate_staff_member(ctx.agency.id, membership_id)
        .await?;

    // ── Revoke OpenFGA agency-membership tuple ─────────────────────────────
    //
    // This mirrors the tuple written during `invite_staff` / `create_staff_user`.
    // Best-effort: a failure is logged but does NOT change the 204 response
    // because the DB deactivation already prevents login.
    if let Some(ref store_id) = ctx.agency.fga_store_id {
        let fga_user = format!("user:{}", member.user_id);
        let fga_object = format!("agency:{}", ctx.agency.id);

        // Map custom roles to base FGA relations to find the correct tuple to delete
        let fga_relation = match member.role.as_str() {
            "admin" => "admin",
            "manager" => "manager",
            _ => "agent",
        };

        match state
            .openfga
            .delete_tuple(store_id, &fga_user, fga_relation, &fga_object)
            .await
        {
            Ok(()) => {
                tracing::info!(
                    user_id = %member.user_id,
                    agency_id = %ctx.agency.id,
                    role = %member.role,
                    fga_rel = %fga_relation,
                    store_id = %store_id,
                    "FGA agency-membership tuple revoked for deactivated staff",
                );
            }
            Err(e) => {
                // Non-fatal — DB membership is already deactivated.
                // The stale FGA tuple poses no security risk without a valid JWT.
                tracing::warn!(
                    user_id   = %member.user_id,
                    agency_id = %ctx.agency.id,
                    role      = %member.role,
                    error     = %e,
                    "FGA tuple deletion failed for deactivated staff member — \
                     tuple can be cleaned up via the admin permissions API",
                );
            }
        }
    } else {
        tracing::warn!(
            agency_id     = %ctx.agency.id,
            membership_id = %membership_id,
            "agency has no FGA store — skipping tuple revocation for deactivated staff",
        );
    }

    Ok(StatusCode::NO_CONTENT)
}
