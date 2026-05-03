use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        helpers::auth_helpers::{generate_temp_password, generate_token},
        ports::{
            auth_port::AuthPort,
            auth_repository::{
                AuthRepository, CreateInviteTokenCommand, CreateMembershipCommand,
                CreateUserCommand, UpsertPortalIndexCommand,
            },
            email_port::EmailPort,
            resident_repository::{CreateResidentCommand, ResidentRepository},
            sms_port::SmsPort,
        },
    },
    domain::{auth::ContactMethod, resident::Resident},
};

pub struct InviteResidentInput {
    pub agency_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub national_id: Option<String>,
    pub portal_base_url: String,
}

pub struct InviteResidentOutput {
    pub resident: Resident,
    pub notified_via: &'static str,
}

pub struct InviteResidentUseCase {
    pub resident_repo: Arc<dyn ResidentRepository>,
    pub auth_repo: Arc<dyn AuthRepository>,
    pub auth_port: Arc<dyn AuthPort>,
    pub email: Arc<dyn EmailPort>,
    pub sms: Arc<dyn SmsPort>,
}

impl InviteResidentUseCase {
    pub async fn execute(
        &self,
        input: InviteResidentInput,
    ) -> Result<InviteResidentOutput, AppError> {
        if input.email.is_none() && input.phone.is_none() {
            return Err(AppError::Validation(
                "Resident must have at least an email or a phone number.".into(),
            ));
        }

        let contact = if let Some(ref e) = input.email {
            ContactMethod::Email(e.to_lowercase())
        } else {
            ContactMethod::parse(input.phone.as_ref().unwrap())
        };

        let already_exists = self
            .auth_repo
            .contact_exists_for_agency(
                input.agency_id,
                contact.value(),
                contact.type_str(),
                "resident",
            )
            .await?;
        if already_exists {
            return Err(AppError::Validation(format!(
                "A resident with {} '{}' already exists in this agency.",
                contact.type_str(),
                contact.value()
            )));
        }

        // 1. Create platform user (inactive until invite accepted)
        let user_id = Uuid::new_v4();
        let (password_hash, temp_plain, must_change, is_active) = match &contact {
            ContactMethod::Phone(_) => {
                let p = generate_temp_password();
                let h = self.auth_port.hash_password(&p).await?;
                (h, Some(p), true, true)
            }
            ContactMethod::Email(_) => ("".to_string(), None, false, false),
        };

        self.auth_repo
            .create_user(CreateUserCommand {
                id: user_id,
                email: input.email.clone().map(|e| e.to_lowercase()),
                phone: input
                    .phone
                    .clone()
                    .map(|p| ContactMethod::parse(&p).value().to_owned()),
                password_hash,
                is_active,
                must_change_password: must_change,
            })
            .await?;

        // 2. Create membership
        let membership_id = self
            .auth_repo
            .create_membership(CreateMembershipCommand {
                user_id,
                agency_id: input.agency_id,
                role: "resident".to_string(),
            })
            .await?;

        // 3. Insert portal index
        self.auth_repo
            .upsert_portal_index(UpsertPortalIndexCommand {
                contact: contact.value().to_owned(),
                contact_type: contact.type_str().to_owned(),
                portal: "resident".to_owned(),
                agency_id: input.agency_id,
                user_id,
                membership_id,
            })
            .await?;

        // 4. Create tenant profile (resident row)
        let resident = self
            .resident_repo
            .create(CreateResidentCommand {
                agency_id: input.agency_id,
                user_id,
                first_name: input.first_name.clone(),
                last_name: input.last_name.clone(),
                email: input.email.clone(),
                phone: input.phone.clone(),
                national_id: input.national_id.clone(),
            })
            .await?;

        // 5. Invite token
        let token = generate_token();
        self.auth_repo
            .create_invite_token(CreateInviteTokenCommand {
                token: token.clone(),
                user_id,
                agency_id: input.agency_id,
                role: "resident".to_string(),
                portal: "resident".to_string(),
                contact: contact.value().to_owned(),
                contact_type: contact.type_str().to_owned(),
                temp_password: temp_plain.clone(),
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(48),
            })
            .await?;

        // 6. Notify
        let notified_via = match &contact {
            ContactMethod::Email(email) => {
                let url = format!("{}/invite/{}", input.portal_base_url, token);
                let _ = self
                    .email
                    .send(
                        email,
                        "Welcome to Emakao – Resident Portal Invitation",
                        &email_body(&input.first_name, &url),
                    )
                    .await;
                "email"
            }
            ContactMethod::Phone(phone) => {
                let temp = temp_plain.unwrap_or_default();
                let msg = format!(
                    "Hi {}, you have been invited to the Emakao resident portal at {}. Your temporary password is: {}. Change it after first login.",
                    input.first_name, input.portal_base_url, temp
                );
                let _ = self.sms.send(phone, &msg).await;
                "sms"
            }
        };

        tracing::info!(resident_id = %resident.id, user_id = %user_id, via = notified_via, "resident invited");
        Ok(InviteResidentOutput {
            resident,
            notified_via,
        })
    }
}

fn email_body(first_name: &str, invite_url: &str) -> String {
    format!(
        r#"<p>Hi {first_name},</p>
<p>You have been invited to the <strong>Emakao Resident Portal</strong>.</p>
<p><a href="{invite_url}" style="background:#1a56db;color:white;padding:10px 20px;border-radius:4px;text-decoration:none">Activate your account</a></p>
<p>This link expires in 48 hours.</p>"#
    )
}
