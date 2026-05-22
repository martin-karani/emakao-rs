// src/infrastructure/providers/twilio.rs
//
// Implements `SmsProvider` for Twilio.
// Credentials JSON shape:
//   { "account_sid": "ACxxx", "auth_token": "...", "from_number": "+12345678900" }

use async_trait::async_trait;
use serde::Deserialize;

use super::traits::SmsProvider;

pub struct TwilioProvider {
    account_sid: String,
    auth_token: String,
    from_number: String,
    client: reqwest::Client,
}

impl TwilioProvider {
    pub fn from_creds(creds: serde_json::Value) -> anyhow::Result<Self> {
        let account_sid = creds["account_sid"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Twilio creds: missing account_sid"))?
            .to_owned();
        let auth_token = creds["auth_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Twilio creds: missing auth_token"))?
            .to_owned();
        let from_number = creds["from_number"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Twilio creds: missing from_number"))?
            .to_owned();

        Ok(Self {
            account_sid,
            auth_token,
            from_number,
            client: reqwest::Client::new(),
        })
    }

    fn endpoint(&self) -> String {
        format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        )
    }
}

#[derive(Deserialize)]
struct TwilioResponse {
    sid: String,
}

#[async_trait]
impl SmsProvider for TwilioProvider {
    fn key(&self) -> &'static str {
        "twilio"
    }

    async fn send(&self, to: &str, body: &str, sender_id: Option<&str>) -> anyhow::Result<String> {
        let from = sender_id.unwrap_or(&self.from_number);

        let params = [("To", to), ("From", from), ("Body", body)];

        let resp = self
            .client
            .post(self.endpoint())
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Twilio SMS error {status}: {text}");
        }

        let parsed: TwilioResponse = resp.json().await?;
        tracing::debug!(to, sid = %parsed.sid, "SMS sent via Twilio");
        Ok(parsed.sid)
    }
}
