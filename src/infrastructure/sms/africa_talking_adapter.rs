use async_trait::async_trait;
use serde::Deserialize;

use crate::{application::{errors::AppError, ports::sms_port::SmsPort}};

pub struct AfricasTalkingSms {
    api_key: String,
    username: String,
    sender_id: Option<String>,
    client: reqwest::Client,
}

impl AfricasTalkingSms {
    pub fn new(api_key: String, username: String, sender_id: Option<String>) -> Self {
        Self {
            api_key,
            username,
            sender_id,
            client: reqwest::Client::new(),
        }
    }

    fn base_url(&self) -> &str {
        if self.username == "sandbox" {
            "https://api.sandbox.africastalking.com/version1/messaging"
        } else {
            "https://api.africastalking.com/version1/messaging"
        }
    }
}

#[derive(Deserialize, Debug)]
struct AtResponse {
    #[serde(rename = "SMSMessageData")]
    sms_message_data: AtMessageData,
}

#[derive(Deserialize, Debug)]
struct AtMessageData {
    #[serde(rename = "Message")]
    message: String,
}

#[async_trait]
impl SmsPort for AfricasTalkingSms {
    async fn send(&self, to: &str, message: &str) -> Result<(), AppError> {
        let mut params = vec![
            ("username", self.username.clone()),
            ("to", to.to_string()),
            ("message", message.to_string()),
        ];

        if let Some(ref sid) = self.sender_id {
            params.push(("from", sid.clone()));
        }

        let resp = self.client
            .post(self.base_url())
            .header("apiKey", &self.api_key)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(
                format!("AT SMS error {status}: {body}"),
            ));
        }

        tracing::debug!(to, "SMS sent via Africa's Talking");
        Ok(())
    }
}