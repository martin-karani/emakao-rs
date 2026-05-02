use garde::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct InviteResidentDto {
    #[garde(length(min = 1, max = 100))]
    pub first_name: String,
    #[garde(length(min = 1, max = 100))]
    pub last_name: String,
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 7, max = 20))]
    pub phone: Option<String>,
    #[garde(length(min = 1, max = 50))]
    pub national_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListResidentsParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
