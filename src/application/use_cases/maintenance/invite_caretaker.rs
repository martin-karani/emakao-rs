use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::{
    errors::AppError,
    helpers::auth_helpers::{generate_temp_password, generate_token},
    notifications::{
        contexts::CaretakerInviteEmailCtx, service::NotificationService, templates::EmailTemplate,
    },
    ports::{
        auth_port::AuthPort,
        auth_repository::{
            AuthRepository, CreateInviteTokenCommand, CreateMembershipCommand, CreateUserCommand,
            UpsertPortalIndexCommand,
        },
        maintenance_repository::MaintenanceRepository,
        property_repository::PropertyRepository,
    },
};
use crate::domain::{
    auth::ContactMethod,
    maintenance::{Caretaker, CreateCaretakerCommand},
};

pub struct InviteCaretakerInput {
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub created_by: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub portal_base_url: String,
    /// Shown in the email greeting, e.g. "Acme Realty". Optional.
    pub agency_name: Option<String>,
}

pub struct InviteCaretakerOutput {
    pub caretaker: Caretaker,
    pub notified_via: &'static str,
}

pub struct InviteCaretakerUseCase {
    pub maintenance_repo: Arc<dyn MaintenanceRepository>,
    pub property_repo: Arc<dyn PropertyRepository>,
    pub auth_repo: Arc<dyn AuthRepository>,
    pub auth_port: Arc<dyn AuthPort>,
    pub notifications: NotificationService,
}

impl InviteCaretakerUseCase {
    pub async fn execute(
        &self,
        input: InviteCaretakerInput,
    ) -> Result<InviteCaretakerOutput, AppError> {
        if input.email.is_none() && input.phone.is_none() {
            return Err(AppError::Validation(
                "Caretaker must have at least an email or a phone number.".into(),
            ));
        }

        let property = self
            .property_repo
            .find_by_id(input.agency_id, input.property_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("property {}", input.property_id)))?;

        let contact = if let Some(ref e) = input.email {
            ContactMethod::Email(e.to_lowercase())
        } else {
            ContactMethod::parse(input.phone.as_ref().unwrap())
        };

        // 1. Check if caretaker already exists for this property
        let existing_caretaker = if let Some(ref email) = input.email {
            self.maintenance_repo
                .find_caretaker_by_email(input.property_id, email)
                .await?
        } else if let Some(ref phone) = input.phone {
            self.maintenance_repo
                .find_caretaker_by_phone(input.property_id, phone)
                .await?
        } else {
            None
        };

        if let Some(caretaker) = existing_caretaker {
            return Ok(InviteCaretakerOutput {
                caretaker,
                notified_via: "none",
            });
        }

        // 2. Platform user - Check if they exist globally
        let existing_user = if let Some(ref email) = input.email {
            self.auth_repo
                .find_user_by_email(&email.to_lowercase())
                .await?
        } else if let Some(ref phone) = input.phone {
            let p = ContactMethod::parse(phone).value().to_owned();
            self.auth_repo.find_user_by_phone(&p).await?
        } else {
            None
        };

        let user_id = if let Some(user) = existing_user {
            user.id
        } else {
            let id = Uuid::new_v4();
            let (password_hash, _temp_plain, must_change, is_active) = match &contact {
                ContactMethod::Phone(_) => {
                    let p = generate_temp_password();
                    let h = self.auth_port.hash_password(&p).await?;
                    (h, Some(p), true, true)
                }
                ContactMethod::Email(_) => ("".to_string(), None, false, false),
            };

            self.auth_repo
                .create_user(CreateUserCommand {
                    id,
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
            id
        };

        // 3. Membership - Check if they have one for this agency
        let membership = self
            .auth_repo
            .find_membership(user_id, input.agency_id)
            .await?;
        let membership_id = if let Some(_) = membership {
            let identity = self
                .auth_repo
                .find_portal_identity(contact.value(), contact.type_str(), "staff")
                .await?;
            if let Some(id) = identity {
                id.membership_id
            } else {
                self.auth_repo
                    .create_membership(CreateMembershipCommand {
                        user_id,
                        agency_id: input.agency_id,
                        role: "caretaker".to_string(),
                    })
                    .await?
            }
        } else {
            self.auth_repo
                .create_membership(CreateMembershipCommand {
                    user_id,
                    agency_id: input.agency_id,
                    role: "caretaker".to_string(),
                })
                .await?
        };

        // 4. Portal index
        self.auth_repo
            .upsert_portal_index(UpsertPortalIndexCommand {
                contact: contact.value().to_owned(),
                contact_type: contact.type_str().to_owned(),
                portal: "staff".to_owned(), // Caretakers use the staff portal
                agency_id: input.agency_id,
                user_id,
                membership_id,
            })
            .await?;

        // 5. Create agency profile (caretaker row)
        let caretaker = self
            .maintenance_repo
            .create_caretaker(CreateCaretakerCommand {
                property_id: input.property_id,
                created_by: input.created_by,
                first_name: input.first_name.clone(),
                last_name: input.last_name.clone(),
                email: input.email.clone(),
                phone: input.phone.clone(),
            })
            .await?;

        // Link the caretaker to the user
        self.maintenance_repo
            .update_caretaker(crate::domain::maintenance::UpdateCaretakerCommand {
                id: caretaker.id,
                user_id: Some(user_id),
                first_name: None,
                last_name: None,
                phone: None,
                email: None,
                is_active: None,
            })
            .await?;

        // 6. Invite token
        let token = generate_token();
        self.auth_repo
            .create_invite_token(CreateInviteTokenCommand {
                token: token.clone(),
                user_id,
                agency_id: input.agency_id,
                role: "caretaker".to_string(),
                portal: "staff".to_string(),
                contact: contact.value().to_owned(),
                contact_type: contact.type_str().to_owned(),
                temp_password: match &contact {
                    ContactMethod::Phone(_) => {
                        let p = generate_temp_password();
                        Some(p)
                    }
                    ContactMethod::Email(_) => None,
                },
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(48),
            })
            .await?;

        // 7. Notify via NotificationService
        let notified_via = match &contact {
            ContactMethod::Email(email) => {
                let _ = self
                    .notifications
                    .email(
                        email.clone(),
                        EmailTemplate::CaretakerInvite,
                        CaretakerInviteEmailCtx {
                            first_name: input.first_name.clone(),
                            property_name: property.name.clone(),
                            invite_url: format!("{}/invite/{}", input.portal_base_url, token),
                            agency_name: input.agency_name.clone(),
                        },
                    )
                    .await;
                "email"
            }
            ContactMethod::Phone(_) => {
                // We don't have a CaretakerInviteSmsCtx yet, skip or use generic
                "none"
            }
        };

        tracing::info!(
            caretaker_id = %caretaker.id,
            user_id      = %user_id,
            via          = notified_via,
            "caretaker invited"
        );
        Ok(InviteCaretakerOutput {
            caretaker,
            notified_via,
        })
    }
}
