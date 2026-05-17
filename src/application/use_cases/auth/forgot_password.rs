use std::sync::Arc;

use crate::application::{
    errors::AppError,
    helpers::auth_helpers,
    notifications::{
        contexts::PasswordResetCtx, service::NotificationService, templates::EmailTemplate,
    },
    ports::auth_repository::AuthRepository,
};

pub struct ForgotPasswordUseCase {
    pub auth_repo: Arc<dyn AuthRepository>,
    pub notifications: NotificationService,
    pub app_base_url: String,
}

pub struct ForgotPasswordInput {
    pub email: String,
}

impl ForgotPasswordUseCase {
    pub fn new(
        auth_repo: Arc<dyn AuthRepository>,
        notifications: NotificationService,
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

        let user = match self.auth_repo.find_user_by_email(&email).await? {
            Some(u) => u,
            None => {
                tracing::info!(email = %email, "password reset requested for unknown email");
                return Ok(());
            }
        };

        let raw_token = auth_helpers::generate_token();
        let token_hash = auth_helpers::sha256_hex(&raw_token);
        let expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(1);

        self.auth_repo
            .save_password_reset_token(user.id, &token_hash, expires_at)
            .await?;

        let reset_url = format!(
            "{}/auth/reset-password?token={}",
            self.app_base_url.trim_end_matches('/'),
            raw_token
        );

        // Soft failure — log and continue so the caller always gets 200
        // (preserves the anti-enumeration guarantee).
        if let Err(e) = self
            .notifications
            .email(
                &email,
                EmailTemplate::PasswordReset,
                PasswordResetCtx {
                    // The jinja template uses `first_name`.
                    // StoredUser doesn't have a name field, so we use email prefix or "User".
                    first_name: email.split('@').next().unwrap_or("User").to_string(),
                    // Previously passed as `reset_link` — jinja template
                    // expects `reset_url`.
                    reset_url,
                },
            )
            .await
        {
            tracing::error!(
                err     = %e,
                user_id = %user.id,
                "failed to enqueue password reset email"
            );
        }

        tracing::info!(user_id = %user.id, "password reset email enqueued");
        Ok(())
    }
}

// ── Reset password (consume the token) ───────────────────────────────────────

pub mod reset {
    use std::sync::Arc;

    use crate::application::{
        errors::AppError, helpers::auth_helpers, ports::auth_port::AuthPort,
        ports::auth_repository::AuthRepository,
    };

    pub struct ResetPasswordUseCase {
        pub auth_repo: Arc<dyn AuthRepository>,
        pub auth_port: Arc<dyn AuthPort>,
    }

    pub struct ResetPasswordInput {
        pub token: String,
        pub new_password: String,
    }

    impl ResetPasswordUseCase {
        pub fn new(auth_repo: Arc<dyn AuthRepository>, auth_port: Arc<dyn AuthPort>) -> Self {
            Self {
                auth_repo,
                auth_port,
            }
        }

        pub async fn execute(&self, input: ResetPasswordInput) -> Result<(), AppError> {
            if input.new_password.len() < 8 {
                return Err(AppError::Validation(
                    "password must be at least 8 characters".into(),
                ));
            }

            let token_hash = auth_helpers::sha256_hex(&input.token);

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

            let password_hash = self.auth_port.hash_password(&input.new_password).await?;

            self.auth_repo
                .reset_password(reset_row.user_id, &password_hash, &token_hash)
                .await?;

            tracing::info!(user_id = %reset_row.user_id, "password reset completed");
            Ok(())
        }
    }
}
