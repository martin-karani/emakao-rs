use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateOwnerDto {
    #[garde(length(min = 1, max = 100))]
    pub first_name: String,
    #[garde(length(min = 1, max = 100))]
    pub last_name: String,
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 7, max = 20))]
    pub phone: Option<String>,
    #[garde(length(min = 1, max = 200))]
    pub company_name: Option<String>,
    #[garde(length(min = 1, max = 20))]
    pub kra_pin: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub bank_name: Option<String>,
    #[garde(length(min = 1, max = 50))]
    pub bank_account: Option<String>,
    #[garde(length(min = 10, max = 15))]
    pub mpesa_number: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateOwnerDto {
    #[garde(length(min = 1, max = 100))]
    pub first_name: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub last_name: Option<String>,
    #[garde(length(min = 7, max = 20))]
    pub phone: Option<String>,
    #[garde(length(min = 1, max = 200))]
    pub company_name: Option<String>,
    #[garde(length(min = 1, max = 20))]
    pub kra_pin: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub bank_name: Option<String>,
    #[garde(length(min = 1, max = 50))]
    pub bank_account: Option<String>,
    #[garde(length(min = 10, max = 15))]
    pub mpesa_number: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AssignOwnerDto {
    #[garde(skip)]
    pub owner_id: Uuid,
    #[garde(custom(validate_ownership))]
    pub ownership_percent: Decimal,
}

fn validate_ownership(v: &Decimal, _ctx: &()) -> garde::Result {
    let min = Decimal::from_f64_retain(0.01).unwrap();
    let max = Decimal::from(100);
    if *v < min || *v > max {
        return Err(garde::Error::new(
            "ownership percent must be between 0.01 and 100",
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListOwnersParams {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
