use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ListTemplatesQuery {
    pub property_id: Option<Uuid>,
    pub channel: Option<String>,
    pub event_key: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertTemplateRequest {
    pub property_id: Option<Uuid>,
    #[serde(default = "default_locale")]
    pub locale: String,
    pub subject: Option<String>,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteTemplateQuery {
    pub property_id: Option<Uuid>,
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "en".into()
}
