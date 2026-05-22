// src/infrastructure/providers/sendgrid.rs
//
// Implements `EmailProvider` for SendGrid v3 Mail Send API.
// Credentials JSON shape:
//   { "api_key": "SG.xxx", "from_email": "noreply@agency.co.ke", "from_name": "BlissVilla" }

use async_trait::async_trait;

use super::traits::{EmailMessage, EmailProvider};

pub struct SendGridProvider {
    api_key: String,
    from_email: String,
    from_name: String,
    client: reqwest::Client,
}

impl SendGridProvider {
    pub fn from_creds(creds: serde_json::Value) -> anyhow::Result<Self> {
        let api_key = creds["api_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("SendGrid creds: missing api_key"))?
            .to_owned();
        let from_email = creds["from_email"]
            .as_str()
            .unwrap_or("noreply@emakao.co.ke")
            .to_owned();
        let from_name = creds["from_name"].as_str().unwrap_or("Emakao").to_owned();

        Ok(Self {
            api_key,
            from_email,
            from_name,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl EmailProvider for SendGridProvider {
    fn key(&self) -> &'static str {
        "sendgrid"
    }

    async fn send(&self, msg: EmailMessage) -> anyhow::Result<String> {
        let body = serde_json::json!({
            "personalizations": [{
                "to": [{ "email": msg.to }]
            }],
            "from": {
                "email": self.from_email,
                "name":  self.from_name,
            },
            "reply_to": msg.reply_to.as_ref().map(|r| serde_json::json!({ "email": r })),
            "subject": msg.subject,
            "content": [
                {
                    "type":  "text/html",
                    "value": msg.body,
                }
            ],
        });

        let resp = self
            .client
            .post("https://api.sendgrid.com/v3/mail/send")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("SendGrid error {status}: {text}");
        }

        // SendGrid returns 202 Accepted with no body; use the X-Message-Id header.
        let message_id = resp
            .headers()
            .get("x-message-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_owned();

        tracing::debug!(to = msg.to, message_id, "email sent via SendGrid");
        Ok(message_id)
    }
}
