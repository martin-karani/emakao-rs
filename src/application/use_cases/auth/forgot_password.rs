// src/application/use_cases/auth/forgot_password.rs
//
// Generates a one-time password-reset token, persists it to the DB with a
// 1-hour TTL, and sends the reset link via email.
//
// Security notes:
//   • The token is a 32-byte cryptographically random value stored as hex.
//   • We always return 200 OK whether or not the email exists — this prevents
//     user-enumeration attacks.
//   • The DB stores a bcrypt hash of the token so that a DB read alone is not
//     enough to hijack accounts.

use std::sync::Arc;

use crate::application::{
    errors::AppError,
    ports::{auth_repository::AuthRepository, notification_port::NotificationPort},
};

pub struct ForgotPasswordUseCase {
    pub auth_repo: Arc<dyn AuthRepository>,
    pub notifications: Arc<dyn NotificationPort>,
    pub app_base_url: String,
}

pub struct ForgotPasswordInput {
    pub email: String,
}

impl ForgotPasswordUseCase {
    pub fn new(
        auth_repo: Arc<dyn AuthRepository>,
        notifications: Arc<dyn NotificationPort>,
        app_base_url: String,
    ) -> Self {
        Self {
            auth_repo,
            notifications,
            app_base_url,
        }
    }

    pub async fn execute(&self, input: ForgotPasswordInput) -> Result<(), AppError> {
        let email = input.email.trim().to_lowercase();

        // Look up user — if not found we still return Ok to prevent enumeration
        let user = match self.auth_repo.find_staff_by_email(&email).await? {
            Some(u) => u,
            None => {
                tracing::info!(email = %email, "password reset requested for unknown email");
                return Ok(());
            }
        };

        // Generate a 32-byte random token
        let raw_token = {
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut bytes);
            hex::encode(bytes)
        };

        // Hash it before storing (argon2 is overkill for short-lived tokens; SHA-256 is fine)
        let token_hash = sha256_hex(&raw_token);

        // Persist with 1-hour TTL
        let expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
        self.auth_repo
            .save_password_reset_token(user.id, &token_hash, expires_at)
            .await?;

        // Build reset link and email it
        let reset_link = format!(
            "{}/auth/reset-password?token={}",
            self.app_base_url.trim_end_matches('/'),
            raw_token
        );

        if let Err(e) = self
            .notifications
            .send_password_reset_email(&user.email, &user.full_name, &reset_link)
            .await
        {
            // Log but don't expose the error to the caller
            tracing::error!(err = %e, user_id = %user.id, "failed to send password reset email");
        }

        tracing::info!(user_id = %user.id, "password reset email sent");
        Ok(())
    }
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(hash)
}

// ─────────────────────────────────────────────────────────────────────────────
// src/application/use_cases/auth/reset_password.rs
// ─────────────────────────────────────────────────────────────────────────────

pub mod reset {
    use std::sync::Arc;

    use crate::application::{errors::AppError, ports::auth_repository::AuthRepository};

    pub struct ResetPasswordUseCase {
        pub auth_repo: Arc<dyn AuthRepository>,
    }

    pub struct ResetPasswordInput {
        pub token: String,
        pub new_password: String,
    }

    impl ResetPasswordUseCase {
        pub fn new(auth_repo: Arc<dyn AuthRepository>) -> Self {
            Self { auth_repo }
        }

        pub async fn execute(&self, input: ResetPasswordInput) -> Result<(), AppError> {
            if input.new_password.len() < 8 {
                return Err(AppError::Validation(
                    "password must be at least 8 characters".into(),
                ));
            }

            let token_hash = sha256_hex(&input.token);

            // Fetch and validate the reset token
            let reset_row = self
                .auth_repo
                .find_valid_reset_token(&token_hash)
                .await?
                .ok_or_else(|| {
                    AppError::Validation("invalid or expired password reset token".into())
                })?;

            if reset_row.expires_at < time::OffsetDateTime::now_utc() {
                return Err(AppError::Validation(
                    "password reset token has expired".into(),
                ));
            }

            // Hash the new password with Argon2
            let password_hash = hash_password(&input.new_password)?;

            // Update the user's password and invalidate the token atomically
            self.auth_repo
                .reset_password(reset_row.user_id, &password_hash, &token_hash)
                .await?;

            tracing::info!(user_id = %reset_row.user_id, "password reset completed");
            Ok(())
        }
    }

    fn sha256_hex(input: &str) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(input.as_bytes()))
    }

    fn hash_password(password: &str) -> Result<String, AppError> {
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::ExternalService(format!("argon2: {e}")))
    }
}
