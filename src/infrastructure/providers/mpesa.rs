// src/infrastructure/providers/mpesa.rs
//
// Implements `PaymentProvider` for M-Pesa Daraja 2.1 (STK Push).
// Wraps the existing `MpesaAdapter` so existing code is unchanged.
//
// Credentials JSON shape:
//   {
//     "consumer_key":    "...",
//     "consumer_secret": "...",
//     "passkey":         "..."
//   }
//
// Settings JSON shape (non-secret, stored unencrypted in `agency_integrations.settings`):
//   {
//     "shortcode":    "174379",
//     "callback_url": "https://api.emakao.co.ke/webhooks/mpesa/{agency_id}",
//     "base_url":     "https://api.safaricom.co.ke"
//   }

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Deserialize;
use time::OffsetDateTime;

use super::traits::{PaymentProvider, PaymentStatus, StkRequest, StkResponse};

pub struct MpesaProvider {
    consumer_key: String,
    consumer_secret: String,
    shortcode: String,
    passkey: String,
    initiator_name: Option<String>,
    security_credential: Option<String>,
    callback_url: String,
    base_url: String,
    client: reqwest::Client,
}

impl MpesaProvider {
    pub fn from_creds(
        creds: serde_json::Value,
        settings: serde_json::Value,
    ) -> anyhow::Result<Self> {
        let consumer_key = creds["consumer_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("M-Pesa creds: missing consumer_key"))?
            .to_owned();
        let consumer_secret = creds["consumer_secret"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("M-Pesa creds: missing consumer_secret"))?
            .to_owned();
        let passkey = creds["passkey"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("M-Pesa creds: missing passkey"))?
            .to_owned();

        let initiator_name = creds["initiator_name"].as_str().map(|s| s.to_owned());
        let security_credential = creds["security_credential"].as_str().map(|s| s.to_owned());

        let shortcode = settings["shortcode"]
            .as_str()
            .unwrap_or("174379")
            .to_owned();
        let callback_url = settings["callback_url"]
            .as_str()
            .unwrap_or("https://api.emakao.co.ke/webhooks/mpesa")
            .to_owned();
        let base_url = settings["base_url"]
            .as_str()
            .unwrap_or("https://api.safaricom.co.ke")
            .to_owned();

        Ok(Self {
            consumer_key,
            consumer_secret,
            shortcode,
            passkey,
            initiator_name,
            security_credential,
            callback_url,
            base_url,
            client: reqwest::Client::new(),
        })
    }

    async fn access_token(&self) -> anyhow::Result<String> {
        let credentials = B64.encode(format!("{}:{}", self.consumer_key, self.consumer_secret));

        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
        }

        let resp: TokenResp = self
            .client
            .get(format!(
                "{}/oauth/v1/generate?grant_type=client_credentials",
                self.base_url
            ))
            .header("Authorization", format!("Basic {credentials}"))
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.access_token)
    }
}

#[async_trait]
impl PaymentProvider for MpesaProvider {
    fn key(&self) -> &'static str {
        "mpesa"
    }

    async fn initiate_stk(&self, req: StkRequest) -> anyhow::Result<StkResponse> {
        let token = self.access_token().await?;

        let ts = OffsetDateTime::now_utc()
            .format(
                &time::format_description::parse("[year][month][day][hour][minute][second]")
                    .unwrap(),
            )
            .unwrap();

        let password = B64.encode(format!("{}{}{}", self.shortcode, self.passkey, ts));

        let amount_u64: u64 = req
            .amount_kes
            .try_into()
            .unwrap_or_else(|_| req.amount_kes.to_string().parse().unwrap_or(0));

        let body = serde_json::json!({
            "BusinessShortCode": self.shortcode,
            "Password":          password,
            "Timestamp":         ts,
            "TransactionType":   "CustomerPayBillOnline",
            "Amount":            amount_u64,
            "PartyA":            req.phone,
            "PartyB":            self.shortcode,
            "PhoneNumber":       req.phone,
            "CallBackURL":       self.callback_url,
            "AccountReference":  req.account_ref,
            "TransactionDesc":   req.description,
        });

        #[derive(Deserialize)]
        struct StkResp {
            #[serde(rename = "MerchantRequestID")]
            merchant_request_id: String,
            #[serde(rename = "CheckoutRequestID")]
            checkout_request_id: String,
            #[serde(rename = "ResponseCode")]
            response_code: String,
        }

        let resp: StkResp = self
            .client
            .post(format!("{}/mpesa/stkpush/v1/processrequest", self.base_url))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if resp.response_code != "0" {
            anyhow::bail!("M-Pesa STK push rejected: code {}", resp.response_code);
        }

        Ok(StkResponse {
            checkout_request_id: resp.checkout_request_id,
            merchant_request_id: resp.merchant_request_id,
        })
    }

    async fn query_status(&self, checkout_id: &str) -> anyhow::Result<PaymentStatus> {
        let token = self.access_token().await?;

        let ts = OffsetDateTime::now_utc()
            .format(
                &time::format_description::parse("[year][month][day][hour][minute][second]")
                    .unwrap(),
            )
            .unwrap();

        let password = B64.encode(format!("{}{}{}", self.shortcode, self.passkey, ts));

        let body = serde_json::json!({
            "BusinessShortCode": self.shortcode,
            "Password":          password,
            "Timestamp":         ts,
            "CheckoutRequestID": checkout_id,
        });

        #[derive(Deserialize)]
        struct QueryResp {
            #[serde(rename = "ResultCode")]
            result_code: String,
        }

        let resp: QueryResp = self
            .client
            .post(format!("{}/mpesa/stkpushquery/v1/query", self.base_url))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        Ok(match resp.result_code.as_str() {
            "0" => PaymentStatus::Success,
            "1032" => PaymentStatus::Cancelled,
            "1" => PaymentStatus::Failed,
            _ => PaymentStatus::Pending,
        })
    }

    async fn verify_receipt(&self, receipt_number: &str, claim_id: uuid::Uuid, agency_id: uuid::Uuid) -> anyhow::Result<bool> {
        let token = self.access_token().await?;

        let initiator = self.initiator_name.as_deref().unwrap_or("initiator");
        // Option B: Assume `security_credential` is already the base64-encrypted password stored in DB
        let security_credential = self.security_credential.as_deref().unwrap_or("");

        let base_url = &self.base_url;
        
        // Emakao expects the environment variable BASE_URL or similar, but for simplicity
        // we can derive it from callback_url which usually looks like "https://api.emakao.co.ke/api/v1/webhooks/mpesa"
        // Let's assume the base domain is the same.
        let parsed_callback = url::Url::parse(&self.callback_url).unwrap_or_else(|_| url::Url::parse("https://api.emakao.co.ke").unwrap());
        let result_url = format!("{}://{}/api/v1/webhooks/mpesa/transaction-status/{}", parsed_callback.scheme(), parsed_callback.host_str().unwrap_or("api.emakao.co.ke"), agency_id);

        let body = serde_json::json!({
            "Initiator": initiator,
            "SecurityCredential": security_credential,
            "CommandID": "TransactionStatusQuery",
            "TransactionID": receipt_number,
            "PartyA": self.shortcode,
            "IdentifierType": "4",
            "ResultURL": result_url,
            "QueueTimeOutURL": result_url,
            "Remarks": claim_id.to_string(), // Store claim_id in Remarks so webhook can use it
            "Occasion": ""
        });

        #[derive(Deserialize)]
        struct DarajaQueryResp {
            #[serde(rename = "ResponseCode")]
            response_code: String,
            #[serde(rename = "ResponseDescription")]
            response_description: Option<String>,
        }

        let resp_res = self
            .client
            .post(format!("{}/mpesa/transactionstatus/v1/query", base_url))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await;

        match resp_res {
            Ok(resp) => {
                if resp.status().is_success() {
                    let parsed: DarajaQueryResp = resp.json().await?;
                    if parsed.response_code == "0" {
                        tracing::info!(receipt = %receipt_number, "Transaction Status query accepted by Safaricom");
                        Ok(true)
                    } else {
                        tracing::warn!(
                            receipt = %receipt_number,
                            code = %parsed.response_code,
                            desc = ?parsed.response_description,
                            "Transaction Status query rejected by Safaricom"
                        );
                        Ok(false)
                    }
                } else {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    tracing::error!(status = %status, text = %text, "Transaction Status API error");
                    Ok(false)
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to send Transaction Status request");
                Err(e.into())
            }
        }
    }
}
