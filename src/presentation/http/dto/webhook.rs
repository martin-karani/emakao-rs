use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct MpesaCallback {
    #[serde(rename = "Body")]
    pub body: MpesaCallbackBody,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MpesaCallbackBody {
    #[serde(rename = "stkCallback")]
    pub stk_callback: StkCallback,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StkCallback {
    #[serde(rename = "MerchantRequestID")]
    pub merchant_request_id: String,
    #[serde(rename = "CheckoutRequestID")]
    pub checkout_request_id: String,
    #[serde(rename = "ResultCode")]
    pub result_code: i32,
    #[serde(rename = "ResultDesc")]
    pub result_desc: String,
    #[serde(rename = "CallbackMetadata")]
    pub callback_metadata: Option<CallbackMetadata>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CallbackMetadata {
    #[serde(rename = "Item")]
    pub item: Vec<CallbackItem>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CallbackItem {
    #[serde(rename = "Name")]
    pub name: String,
    #[schema(value_type = Object, nullable = true)]
    #[serde(rename = "Value")]
    pub value: Option<serde_json::Value>,
}

impl CallbackMetadata {
    pub fn get(&self, name: &str) -> Option<&serde_json::Value> {
        self.item
            .iter()
            .find(|i| i.name == name)
            .and_then(|i| i.value.as_ref())
    }
}
