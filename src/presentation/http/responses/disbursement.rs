// src/presentation/http/responses/disbursement.rs
//
// DisbursementResponse is defined in `responses::owner` (where it lives
// alongside OwnerResponse, since disbursements are displayed on the owner
// detail page). This module re-exports it so that `disbursement_routes.rs`
// and the disbursement handler can import from a canonical, domain-aligned path.

pub use crate::presentation::http::responses::owner::DisbursementResponse;
