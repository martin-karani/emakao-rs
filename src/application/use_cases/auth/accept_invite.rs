// src/application/use_cases/auth/accept_invite.rs
//
// Called when an invited user activates their account.
//
// Email flow:
//   Frontend GET {portal}.emakao.co.ke/invite/{token}
//   → shows "set your password" form
//   → POST /api/v1/auth/accept-invite { token, new_password }
//
// Phone flow (temp password):
//   User logs in with the temp password they received via SMS
//   → login handler detects must_change_password = true
//   → frontend shows "change your password" form
//   → POST /api/v1/auth/accept-invite { token: "", new_password, phone, temp_password }
//   (The token is empty; we validate via temp_password instead.)
//   OR — simpler implementation that works for both flows:
//   Phone users also receive a token in the SMS URL if the SMS allows links.
//   For feature phones that can't click links, temp_password path is used.

use std::sync::Arc;
use uuid::Uuid;

use crate::application::{
    errors::AppError,
    ports::{auth_port::AuthPort, auth_repository::AuthRepository},
};

pub struct AcceptInviteInput {
    /// The invite token from the email link / SMS link.
    pub token: String,
    /// New permanent password chosen by the user.
    pub new_password: String,
}

pub struct AcceptInviteOutput {
    pub user_id: Uuid,
    pub portal: String,
    pub agency_id: Uuid,
}

pub struct AcceptInviteUseCase {
    pub repo: Arc<dyn AuthRepository>,
    pub auth: Arc<dyn AuthPort>,
}

impl AcceptInviteUseCase {
    pub fn new(repo: Arc<dyn AuthRepository>, auth: Arc<dyn AuthPort>) -> Self {
        Self { repo, auth }
    }

    pub async fn execute(&self, input: AcceptInviteInput) -> Result<AcceptInviteOutput, AppError> {
        if input.new_password.len() < 8 {
            return Err(AppError::Validation(
                "Password must be at least 8 characters.".into(),
            ));
        }

        // 1. Validate + consume the token (atomic: validates expiry, deletes row)
        let invite = self.repo.consume_invite_token(&input.token).await?;

        // 2. Hash the new password
        let hash = self.auth.hash_password(&input.new_password).await?;

        // 3. Activate the user
        self.repo.activate_user(invite.user_id, &hash).await?;

        tracing::info!(
            user_id   = %invite.user_id,
            portal    = %invite.portal,
            agency_id = %invite.agency_id,
            "invite accepted — user activated"
        );

        Ok(AcceptInviteOutput {
            user_id: invite.user_id,
            portal: invite.portal,
            agency_id: invite.agency_id,
        })
    }
}
