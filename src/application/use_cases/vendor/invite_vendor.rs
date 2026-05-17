use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::{
    errors::AppError,
    helpers::auth_helpers::{generate_temp_password, generate_token},
    notifications::{
        contexts::{VendorInviteEmailCtx, VendorInviteSmsCtx},
        service::NotificationService,
        templates::{EmailTemplate, SmsTemplate},
    },
    ports::{
        auth_port::AuthPort,
        auth_repository::{
            AuthRepository, CreateInviteTokenCommand, CreateMembershipCommand, CreateUserCommand,
            UpsertPortalIndexCommand,
        },
        vendor_repository::VendorRepository,
    },
};
use crate::domain::{
    auth::ContactMethod,
    vendor::{CreateVendorCommand, Vendor},
};

pub struct InviteVendorInput {
    pub agency_id: Uuid,
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub speciality: Option<String>,
    pub notes: Option<String>,
    pub portal_base_url: String,
    /// Shown in the email, e.g. "Acme Realty". Optional.
    pub agency_name: Option<String>,
}

pub struct InviteVendorUseCase {
    pub vendor_repo: Arc<dyn VendorRepository>,
    pub auth_repo: Arc<dyn AuthRepository>,
    pub auth_port: Arc<dyn AuthPort>,
    pub notifications: NotificationService,
}

impl InviteVendorUseCase {
    pub fn new(
        vendor_repo: Arc<dyn VendorRepository>,
        auth_repo: Arc<dyn AuthRepository>,
        auth_port: Arc<dyn AuthPort>,
        notifications: NotificationService,
    ) -> Self {
        Self {
            vendor_repo,
            auth_repo,
            auth_port,
            notifications,
        }
    }

    pub async fn execute(&self, input: InviteVendorInput) -> Result<Vendor, AppError> {
        if input.email.is_none() && input.phone.is_none() {
            return Err(AppError::Validation(
                "Vendor must have at least an email or a phone number.".into(),
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
                "vendor",
            )
            .await?;
        if already_exists {
            return Err(AppError::Validation(format!(
                "A vendor with {} '{}' already exists in this agency.",
                contact.type_str(),
                contact.value()
            )));
        }

        let user_id = Uuid::new_v4();

        // 1. Tenant profile
        let vendor = self
            .vendor_repo
            .create(CreateVendorCommand {
                agency_id: input.agency_id,
                user_id: Some(user_id),
                name: input.name.clone(),
                contact_name: input.contact_name.clone(),
                email: input.email.clone(),
                phone: input.phone.clone(),
                speciality: input.speciality.clone(),
                notes: input.notes.clone(),
            })
            .await?;

        // 2. Auth credentials
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

        let membership_id = self
            .auth_repo
            .create_membership(CreateMembershipCommand {
                user_id,
                agency_id: input.agency_id,
                role: "vendor".to_string(),
            })
            .await?;

        self.auth_repo
            .upsert_portal_index(UpsertPortalIndexCommand {
                contact: contact.value().to_owned(),
                contact_type: contact.type_str().to_owned(),
                portal: "vendor".to_owned(),
                agency_id: input.agency_id,
                user_id,
                membership_id,
            })
            .await?;

        let token = generate_token();
        self.auth_repo
            .create_invite_token(CreateInviteTokenCommand {
                token: token.clone(),
                user_id,
                agency_id: input.agency_id,
                role: "vendor".to_string(),
                portal: "vendor".to_string(),
                contact: contact.value().to_owned(),
                contact_type: contact.type_str().to_owned(),
                temp_password: temp_plain.clone(),
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(48),
            })
            .await?;

        // 3. Notify via NotificationService
        match &contact {
            ContactMethod::Email(email) => {
                let _ = self
                    .notifications
                    .email(
                        email.clone(),
                        EmailTemplate::VendorInvite,
                        VendorInviteEmailCtx {
                            contact_name: input.contact_name.clone(),
                            vendor_name: input.name.clone(),
                            invite_url: format!("{}/invite/{}", input.portal_base_url, token),
                            agency_name: input.agency_name.clone(),
                        },
                    )
                    .await;
            }
            ContactMethod::Phone(phone) => {
                let _ = self
                    .notifications
                    .sms(
                        phone.clone(),
                        SmsTemplate::VendorInvite,
                        VendorInviteSmsCtx {
                            portal_url: input.portal_base_url.clone(),
                            temp_password: temp_plain.as_deref().unwrap_or("").to_owned(),
                        },
                    )
                    .await;
            }
        }

        tracing::info!(vendor_id = %vendor.id, user_id = %user_id, "vendor invited");
        Ok(vendor)
    }
}
