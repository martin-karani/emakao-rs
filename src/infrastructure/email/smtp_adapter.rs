use async_trait::async_trait;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

use crate::{application::{errors::AppError, ports::email_port::EmailPort}};

pub struct SmtpEmail {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl SmtpEmail {
    pub fn new(
        host: &str,
        port: u16,
        from: String,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self, AppError> {
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port);

        if let (Some(user), Some(pass)) = (username, password) {
            builder = builder.credentials(Credentials::new(user.into(), pass.into()));
        }

        Ok(Self {
            transport: builder.build(),
            from,
        })
    }
}

#[async_trait]
impl EmailPort for SmtpEmail {
    async fn send(&self, to: &str, subject: &str, html_body: &str) -> Result<(), AppError> {
        let email = Message::builder()
            .from(self.from.parse().map_err(|e: lettre::address::AddressError| {
                AppError::ExternalService(e.to_string())
            })?)
            .to(to.parse().map_err(|e: lettre::address::AddressError| {
                AppError::ExternalService(e.to_string())
            })?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        tracing::debug!(to, subject, "email sent");
        Ok(())
    }
}