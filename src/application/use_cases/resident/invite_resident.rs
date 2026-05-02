use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            email_port::EmailPort,
            resident_repository::{CreateResidentCommand, ResidentRepository},
            sms_port::SmsPort,
        },
    },
    domain::resident::Resident,
};

pub struct InviteResidentUseCase {
    pub repo: Arc<dyn ResidentRepository>,
    pub email: Arc<dyn EmailPort>,
    pub sms: Arc<dyn SmsPort>,
}

pub struct InviteResidentInput {
    pub agency_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub national_id: Option<String>,
}

impl InviteResidentUseCase {
    pub fn new(
        repo: Arc<dyn ResidentRepository>,
        email: Arc<dyn EmailPort>,
        sms: Arc<dyn SmsPort>,
    ) -> Self {
        Self { repo, email, sms }
    }

    pub async fn execute(&self, input: InviteResidentInput) -> Result<Resident, AppError> {
        // Uses: ResidentRepository::find_by_email — guard duplicate
        if self.repo.find_by_email(input.agency_id, &input.email).await?.is_some() {
            return Err(AppError::Validation(format!(
                "resident with email '{}' already exists",
                input.email
            )));
        }

        // Uses: ResidentRepository::create
        let resident = self.repo.create(CreateResidentCommand {
            agency_id: input.agency_id,
            first_name: input.first_name.clone(),
            last_name: input.last_name.clone(),
            email: input.email.clone(),
            phone: input.phone.clone(),
            national_id: input.national_id,
        }).await?;

        // Non-fatal side effects
        let _ = self.email.send(
            &input.email,
            "Welcome to emakao – activate your portal",
            &format!(
                "<p>Hi {},</p><p>You have been invited to the resident portal. \
                 Check your email to activate your account.</p>",
                input.first_name
            ),
        ).await;

        if let Some(ref phone) = input.phone {
            let _ = self.sms.send(
                phone,
                &format!(
                    "Hi {}, you have been invited to the emakao resident portal.",
                    input.first_name
                ),
            ).await;
        }

        tracing::info!(resident_id = %resident.id, "resident invited");
        Ok(resident)
    }
}