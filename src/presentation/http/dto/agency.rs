use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateAgencyDto {
    #[garde(length(min = 2, max = 120))]
    pub name: String,

    /// Lowercase, hyphen-separated, e.g. "acme-realty".
    /// Must be unique; becomes both the URL slug and the Postgres schema prefix.
    #[garde(pattern(r"^[a-z0-9]+(?:-[a-z0-9]+)*$"), length(min = 2, max = 63))]
    pub slug: String,

    #[garde(length(min = 2, max = 2))]
    pub country_code: String,

    #[garde(length(min = 3, max = 3))]
    pub currency_code: String,
}
