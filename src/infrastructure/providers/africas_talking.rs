// src/infrastructure/providers/africas_talking.rs
//
// Implements the new `SmsProvider` trait for Africa's Talking.
// Wraps the same HTTP logic already in
// `src/infrastructure/sms/africa_talking_adapter.rs` so there is no
// duplication — the existing `AfricasTalkingSms` satisfies `SmsPort`
// (application layer), while this struct satisfies `SmsProvider`
// (infrastructure/providers trait used by the per-agency ProviderRegistry).
//
// The two co-exist: `SmsPort` is used by the existing NotificationService
// workers; `SmsProvider` is used by the new NotificationDispatcher.
// Once you are ready to consolidate, replace the old workers and delete SmsPort.

use async_trait::async_trait;
use serde::Deserialize;

use super::traits::SmsProvider;

pub struct AfricasTalkingProvider {
    api_key: String,
    username: String,
    sender_id: Option<String>,
    client: reqwest::Client,
}

impl AfricasTalkingProvider {
    /// Build from decrypted credentials JSON:
    /// `{ "api_key": "...", "username": "...", "sender_id": "EMAKAO" }`
    pub fn from_creds(creds: serde_json::Value) -> anyhow::Result<Self> {
        let api_key = creds["api_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("AT creds: missing api_key"))?
            .to_owned();
        let username = creds["username"].as_str().unwrap_or("sandbox").to_owned();
        let sender_id = creds["sender_id"].as_str().map(str::to_owned);

        Ok(Self {
            api_key,
            username,
            sender_id,
            client: reqwest::Client::new(),
        })
    }

    fn base_url(&self) -> &str {
        if self.username == "sandbox" {
            "https://api.sandbox.africastalking.com/version1/messaging"
        } else {
            "https://api.africastalking.com/version1/messaging"
        }
    }
}

#[derive(Deserialize)]
struct AtResponse {
    #[serde(rename = "SMSMessageData")]
    sms_message_data: AtMessageData,
}

#[derive(Deserialize)]
struct AtMessageData {
    #[serde(rename = "Message")]
    message: String,
}

#[async_trait]
impl SmsProvider for AfricasTalkingProvider {
    fn key(&self) -> &'static str {
        "africas_talking"
    }

    async fn send(&self, to: &str, body: &str, sender_id: Option<&str>) -> anyhow::Result<String> {
        let sid = sender_id.or(self.sender_id.as_deref()).map(str::to_owned);

        let mut params = vec![
            ("username", self.username.clone()),
            ("to", to.to_owned()),
            ("message", body.to_owned()),
        ];
        if let Some(s) = sid {
            params.push(("from", s));
        }

        let resp = self
            .client
            .post(self.base_url())
            .header("apiKey", &self.api_key)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("AT SMS error {status}: {text}");
        }

        let parsed: AtResponse = resp.json().await?;
        tracing::debug!(to, msg = %parsed.sms_message_data.message, "SMS sent via AT");
        Ok(parsed.sms_message_data.message)
    }
}
