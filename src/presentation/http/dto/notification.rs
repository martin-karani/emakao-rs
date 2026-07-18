use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub limit: Option<i64>,
}
