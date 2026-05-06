use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        helpers::auth_helpers::{generate_temp_password, generate_token},
        notifications::{
            service::NotificationService,
            templates::{EmailTemplate, SmsTemplate},
        },
        ports::{
            auth_port::AuthPort,
            auth_repository::{
                AuthRepository, CreateInviteTokenCommand, CreateMembershipCommand,
                CreateUserCommand, UpsertPortalIndexCommand,
            },
            owner_repository::OwnerRepository,
        },
    },
    domain::{
        auth::ContactMethod,
        owner::{CreateOwnerCommand, Owner},
    },
};

pub struct OnboardOwnerInput {
    pub agency_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub kra_pin: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub mpesa_number: Option<String>,
    pub portal_base_url: String,
    /// Shown in the email greeting. Optional.
    pub agency_name: Option<String>,
}

pub struct OnboardOwnerUseCase {
    pub owner_repo: Arc<dyn OwnerRepository>,
    pub auth_repo: Arc<dyn AuthRepository>,
    pub auth_port: Arc<dyn AuthPort>,
    pub notifications: NotificationService,
}

impl OnboardOwnerUseCase {
    pub async fn execute(&self, input: OnboardOwnerInput) -> Result<Owner, AppError> {
        if input.email.is_none() && input.phone.is_none() {
            return Err(AppError::Validation(
                "Owner must have at least an email or a phone number.".into(),
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
                "owner",
            )
            .await?;
        if already_exists {
            return Err(AppError::Validation(format!(
                "An owner with {} '{}' already exists in this agency.",
                contact.type_str(),
                contact.value()
            )));
        }

        // 1. Tenant profile
        let owner = self
            .owner_repo
            .create(CreateOwnerCommand {
                agency_id: input.agency_id,
                user_id: Some(Uuid::new_v4()), // placeholder — real user_id set below
                first_name: input.first_name.clone(),
                last_name: input.last_name.clone(),
                email: input.email.clone(),
                phone: input.phone.clone(),
                company_name: input.company_name.clone(),
                kra_pin: input.kra_pin.clone(),
                bank_name: input.bank_name.clone(),
                bank_account: input.bank_account.clone(),
                mpesa_number: input.mpesa_number.clone(),
            })
            .await?;

        // 2. Platform user
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

        // 3. Membership
        let membership_id = self
            .auth_repo
            .create_membership(CreateMembershipCommand {
                user_id,
                agency_id: input.agency_id,
                role: "owner".to_string(),
            })
            .await?;

        // 4. Portal index
        self.auth_repo
            .upsert_portal_index(UpsertPortalIndexCommand {
                contact: contact.value().to_owned(),
                contact_type: contact.type_str().to_owned(),
                portal: "owner".to_owned(),
                agency_id: input.agency_id,
                user_id,
                membership_id,
            })
            .await?;

        // 5. Invite token
        let token = generate_token();
        self.auth_repo
            .create_invite_token(CreateInviteTokenCommand {
                token: token.clone(),
                user_id,
                agency_id: input.agency_id,
                role: "owner".to_string(),
                portal: "owner".to_string(),
                contact: contact.value().to_owned(),
                contact_type: contact.type_str().to_owned(),
                temp_password: temp_plain.clone(),
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(48),
            })
            .await?;

        // 6. Notify via NotificationService
        match &contact {
            ContactMethod::Email(email) => {
                let invite_url = format!("{}/invite/{}", input.portal_base_url, token);
                let _ = self
                    .notifications
                    .email(
                        email.clone(),
                        EmailTemplate::OwnerInvite,
                        serde_json::json!({
                            "first_name":  input.first_name,
                            "invite_url":  invite_url,
                            "agency_name": input.agency_name,
                        }),
                    )
                    .await;
            }
            ContactMethod::Phone(phone) => {
                let _ = self
                    .notifications
                    .sms(
                        phone.clone(),
                        SmsTemplate::OwnerInvite,
                        serde_json::json!({
                            "first_name":    input.first_name,
                            "portal_url":    input.portal_base_url,
                            "temp_password": temp_plain.as_deref().unwrap_or(""),
                        }),
                    )
                    .await;
            }
        }

        tracing::info!(owner_id = %owner.id, user_id = %user_id, "owner onboarded");
        Ok(owner)
    }
}
