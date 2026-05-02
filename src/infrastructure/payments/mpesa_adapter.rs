//! M-Pesa STK Push (Lipa Na M-Pesa Online) adapter.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::application::errors::AppError;

pub struct MpesaAdapter {
    consumer_key: String,
    consumer_secret: String,
    shortcode: String,
    passkey: String,
    callback_url: String,
    base_url: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct StkPushRequest {
    #[serde(rename = "BusinessShortCode")]
    business_short_code: String,
    #[serde(rename = "Password")]
    password: String,
    #[serde(rename = "Timestamp")]
    timestamp: String,
    #[serde(rename = "TransactionType")]
    transaction_type: String,
    #[serde(rename = "Amount")]
    amount: u64,
    #[serde(rename = "PartyA")]
    party_a: String,
    #[serde(rename = "PartyB")]
    party_b: String,
    #[serde(rename = "PhoneNumber")]
    phone_number: String,
    #[serde(rename = "CallBackURL")]
    callback_url: String,
    #[serde(rename = "AccountReference")]
    account_reference: String,
    #[serde(rename = "TransactionDesc")]
    transaction_desc: String,
}

#[derive(Deserialize, Debug)]
pub struct StkPushResponse {
    #[serde(rename = "MerchantRequestID")]
    pub merchant_request_id: String,
    #[serde(rename = "CheckoutRequestID")]
    pub checkout_request_id: String,
    #[serde(rename = "ResponseCode")]
    pub response_code: String,
    #[serde(rename = "CustomerMessage")]
    pub customer_message: String,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
}

impl MpesaAdapter {
    pub fn new(
        consumer_key: String,
        consumer_secret: String,
        shortcode: String,
        passkey: String,
        callback_url: String,
        base_url: String,
    ) -> Self {
        Self {
            consumer_key,
            consumer_secret,
            shortcode,
            passkey,
            callback_url,
            base_url,
            client: reqwest::Client::new(),
        }
    }

    async fn access_token(&self) -> Result<String, AppError> {
        let credentials = STANDARD.encode(format!("{}:{}", self.consumer_key, self.consumer_secret));
        let resp: TokenResponse = self.client
            .get(format!("{}/oauth/v1/generate?grant_type=client_credentials", self.base_url))
            .header("Authorization", format!("Basic {credentials}"))
            .send()
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?
            .json()
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        Ok(resp.access_token)
    }

    pub async fn stk_push(
        &self,
        phone_number: &str,
        amount_kes: u64,
        account_reference: &str,
        description: &str,
    ) -> Result<StkPushResponse, AppError> {
        let token = self.access_token().await?;
        let timestamp = OffsetDateTime::now_utc()
            .format(&time::format_description::parse("[year][month][day][hour][minute][second]")
                .unwrap())
            .unwrap();

        let password_raw = format!("{}{}{}", self.shortcode, self.passkey, timestamp);
        let password = STANDARD.encode(password_raw.as_bytes());

        let body = StkPushRequest {
            business_short_code: self.shortcode.clone(),
            password,
            timestamp,
            transaction_type: "CustomerPayBillOnline".into(),
            amount: amount_kes,
            party_a: phone_number.to_string(),
            party_b: self.shortcode.clone(),
            phone_number: phone_number.to_string(),
            callback_url: self.callback_url.clone(),
            account_reference: account_reference.to_string(),
            transaction_desc: description.to_string(),
        };

        let resp = self.client
            .post(format!("{}/mpesa/stkpush/v1/processrequest", self.base_url))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("M-Pesa error {status}: {text}")));
        }

        resp.json().await.map_err(|e| AppError::ExternalService(e.to_string()))
    }
}