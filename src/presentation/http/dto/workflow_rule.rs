use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkflowRuleRequest {
    pub name: String,
    pub event_type: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub offset_hours: i32,
    #[serde(default = "empty_object")]
    pub conditions: serde_json::Value,
    #[serde(default = "empty_array")]
    pub actions: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchWorkflowRuleRequest {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub offset_hours: Option<i32>,
    pub conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

fn empty_array() -> serde_json::Value {
    serde_json::json!([])
}
