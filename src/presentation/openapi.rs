use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // ── Health ────────────────────────────────────────────────────────────
        crate::presentation::http::handlers::health::health,
        crate::presentation::http::handlers::health::ready,

        // ── Auth ──────────────────────────────────────────────────────────────
        crate::presentation::http::handlers::auth::login,
        crate::presentation::http::handlers::auth::register,
        crate::presentation::http::handlers::auth::refresh,

        // ── Properties ────────────────────────────────────────────────────────
        crate::presentation::http::handlers::property::list_properties,
        crate::presentation::http::handlers::property::get_property,
        crate::presentation::http::handlers::property::create_property,
        crate::presentation::http::handlers::property::update_property,
        crate::presentation::http::handlers::property::delete_property,

        // ── Residents ─────────────────────────────────────────────────────────
        crate::presentation::http::handlers::resident::list_residents,
        crate::presentation::http::handlers::resident::get_resident,
        crate::presentation::http::handlers::resident::invite_resident,

        // ── Agreements ────────────────────────────────────────────────────────
        crate::presentation::http::handlers::agreement::list_agreements,
        crate::presentation::http::handlers::agreement::get_agreement,
        crate::presentation::http::handlers::agreement::create_agreement,
        crate::presentation::http::handlers::agreement::terminate_agreement,

        // ── Payments ──────────────────────────────────────────────────────────
        crate::presentation::http::handlers::payment::list_claims,
        crate::presentation::http::handlers::payment::submit_claim,
        crate::presentation::http::handlers::payment::review_claim,

        // ── Ledger ────────────────────────────────────────────────────────────
        crate::presentation::http::handlers::ledger::list_entries,
        crate::presentation::http::handlers::ledger::post_charge,
        crate::presentation::http::handlers::ledger::get_balance,

        // ── Maintenance ───────────────────────────────────────────────────────
        crate::presentation::http::handlers::maintenance::list_work_orders,
        crate::presentation::http::handlers::maintenance::get_work_order,
        crate::presentation::http::handlers::maintenance::create_work_order,
        crate::presentation::http::handlers::maintenance::update_work_order,

        // ── Owners ────────────────────────────────────────────────────────────
        crate::presentation::http::handlers::owner::list_owners,
        crate::presentation::http::handlers::owner::get_owner,
        crate::presentation::http::handlers::owner::create_owner,
        crate::presentation::http::handlers::owner::update_owner,
        crate::presentation::http::handlers::owner::assign_owner_to_property,

        // ── Vendors ───────────────────────────────────────────────────────────
        crate::presentation::http::handlers::vendor::list_vendors,
        crate::presentation::http::handlers::vendor::get_vendor,
        crate::presentation::http::handlers::vendor::create_vendor,
        crate::presentation::http::handlers::vendor::update_vendor,

        // ── Utility ───────────────────────────────────────────────────────────
        crate::presentation::http::handlers::utility::list_meters,
        crate::presentation::http::handlers::utility::get_meter,
        crate::presentation::http::handlers::utility::create_meter,
        crate::presentation::http::handlers::utility::record_reading,
        crate::presentation::http::handlers::utility::list_bills,
        crate::presentation::http::handlers::utility::generate_bill,

        // ── Subscription (agency-facing) ──────────────────────────────────────
        crate::presentation::http::handlers::subscription::list_plans,
        crate::presentation::http::handlers::subscription::get_plan,
        crate::presentation::http::handlers::subscription::get_status,
        crate::presentation::http::handlers::subscription::get_entitlements,
        crate::presentation::http::handlers::subscription::get_usage,
        crate::presentation::http::handlers::subscription::initiate_payment,
        crate::presentation::http::handlers::subscription::subscribe,
        crate::presentation::http::handlers::subscription::change_plan,
        crate::presentation::http::handlers::subscription::cancel_subscription,
        crate::presentation::http::handlers::subscription::list_invoices,

        // ── Subscription (platform admin) ─────────────────────────────────────
        crate::presentation::http::handlers::subscription::admin_get_status,
        crate::presentation::http::handlers::subscription::admin_get_entitlements,
        crate::presentation::http::handlers::subscription::admin_change_plan,
        crate::presentation::http::handlers::subscription::admin_set_override,
        crate::presentation::http::handlers::subscription::admin_remove_override,
        crate::presentation::http::handlers::subscription::admin_cancel_subscription,

        // ── Webhooks ──────────────────────────────────────────────────────────
        crate::presentation::http::handlers::webhook::mpesa_callback,
    ),
    components(schemas(
        // ── Shared error ──────────────────────────────────────────────────────
        crate::presentation::error::ErrorResponse,

        // ── Domain enums (needed because response/dto structs reference them) ─
        crate::domain::property::PropertyType,
        crate::domain::property::PropertyConfig,
        crate::domain::property::BuildingClass,
        crate::domain::property::BillingCycle,
        crate::domain::ledger::LedgerEntryType,
        crate::domain::maintenance::WorkOrderPriority,
        crate::domain::maintenance::WorkOrderStatus,
        crate::domain::payment::ClaimStatus,
        crate::domain::payment::PaymentMethodType,
        crate::domain::agreement::BillingFrequency,
        crate::domain::agreement::AgreementStatus,
        crate::domain::utility::MeterType,
        crate::domain::utility::BillingMode,
        crate::domain::utility::UtilityBillStatus,
        crate::domain::vendor::VendorStatus,
        crate::domain::owner::OwnerPortalStatus,
        crate::domain::resident::PortalStatus,

        // ── Auth DTOs / responses ─────────────────────────────────────────────
        crate::presentation::http::dto::auth::LoginDto,
        crate::presentation::http::dto::auth::RegisterDto,
        crate::presentation::http::dto::auth::RefreshDto,
        crate::presentation::http::responses::auth::TokenResponse,

        // ── Property DTOs / responses ─────────────────────────────────────────
        crate::presentation::http::dto::property::CreatePropertyDto,
        crate::presentation::http::dto::property::UpdatePropertyDto,
        crate::presentation::http::dto::property::ListPropertiesParams,
        crate::presentation::http::responses::property::PropertyResponse,

        // ── Resident DTOs / responses ─────────────────────────────────────────
        crate::presentation::http::dto::resident::InviteResidentDto,
        crate::presentation::http::dto::resident::ListResidentsParams,
        crate::presentation::http::responses::resident::ResidentResponse,

        // ── Agreement DTOs / responses ────────────────────────────────────────
        crate::presentation::http::dto::agreement::CreateAgreementDto,
        crate::presentation::http::dto::agreement::ListAgreementsParams,
        crate::presentation::http::responses::agreement::AgreementResponse,

        // ── Payment DTOs / responses ──────────────────────────────────────────
        crate::presentation::http::dto::payment::SubmitClaimDto,
        crate::presentation::http::dto::payment::ReviewClaimDto,
        crate::presentation::http::dto::payment::ListClaimsParams,
        crate::presentation::http::responses::payment::PaymentClaimResponse,

        // ── Ledger DTOs / responses ───────────────────────────────────────────
        crate::presentation::http::dto::ledger::PostChargeDto,
        crate::presentation::http::dto::ledger::ListLedgerParams,
        crate::presentation::http::responses::ledger::LedgerEntryResponse,
        crate::presentation::http::responses::ledger::BalanceSummaryResponse,

        // ── Maintenance DTOs / responses ──────────────────────────────────────
        crate::presentation::http::dto::maintenance::CreateWorkOrderDto,
        crate::presentation::http::dto::maintenance::UpdateWorkOrderDto,
        crate::presentation::http::dto::maintenance::ListWorkOrdersParams,
        crate::presentation::http::responses::maintenance::WorkOrderResponse,

        // ── Owner DTOs / responses ────────────────────────────────────────────
        crate::presentation::http::dto::owner::CreateOwnerDto,
        crate::presentation::http::dto::owner::UpdateOwnerDto,
        crate::presentation::http::dto::owner::AssignOwnerDto,
        crate::presentation::http::dto::owner::ListOwnersParams,
        crate::presentation::http::responses::owner::OwnerResponse,

        // ── Vendor DTOs / responses ───────────────────────────────────────────
        crate::presentation::http::dto::vendor::CreateVendorDto,
        crate::presentation::http::dto::vendor::UpdateVendorDto,
        crate::presentation::http::dto::vendor::ListVendorsParams,
        crate::presentation::http::responses::vendor::VendorResponse,

        // ── Utility DTOs / responses ──────────────────────────────────────────
        crate::presentation::http::dto::utility::CreateMeterDto,
        crate::presentation::http::dto::utility::RecordReadingDto,
        crate::presentation::http::responses::utility::UtilityMeterResponse,
        crate::presentation::http::responses::utility::MeterReadingResponse,
        crate::presentation::http::responses::utility::UtilityBillResponse,

        // ── Subscription DTOs / responses ─────────────────────────────────────
        crate::presentation::http::handlers::subscription::PlanResponse,
        crate::presentation::http::handlers::subscription::SubscriptionStatusResponse,
        crate::presentation::http::handlers::subscription::ChangePlanDto,
        crate::presentation::http::handlers::subscription::CancelDto,
        crate::presentation::http::handlers::subscription::SetOverrideDto,
        crate::presentation::http::handlers::subscription::InitiatePaymentDto,
        crate::presentation::http::handlers::subscription::InitiatePaymentResponse,

        // ── Webhook structs ───────────────────────────────────────────────────
        crate::presentation::http::handlers::webhook::MpesaCallback,
        crate::presentation::http::handlers::webhook::MpesaCallbackBody,
        crate::presentation::http::handlers::webhook::StkCallback,
        crate::presentation::http::handlers::webhook::CallbackMetadata,
        crate::presentation::http::handlers::webhook::CallbackItem,

        // ── Health responses ──────────────────────────────────────────────────
        crate::presentation::http::handlers::health::HealthResponse,
        crate::presentation::http::handlers::health::ReadyResponse,
        crate::presentation::http::handlers::health::ComponentStatus,
    )),
    tags(
        (name = "Health",       description = "Liveness and readiness probes"),
        (name = "Auth",         description = "JWT authentication — login, register, token refresh"),
        (name = "Properties",   description = "Property CRUD"),
        (name = "Residents",    description = "Resident management"),
        (name = "Agreements",   description = "Lease agreements"),
        (name = "Payments",     description = "Payment claim submission and review"),
        (name = "Ledger",       description = "Double-entry financial ledger"),
        (name = "Maintenance",  description = "Work orders"),
        (name = "Owners",       description = "Owner portal"),
        (name = "Vendors",      description = "Vendor management"),
        (name = "Utility",      description = "Utility meters, readings and bills"),
        (name = "Subscription", description = "Subscription plans and billing (agency-facing)"),
        (name = "Admin",        description = "Platform-admin subscription management"),
        (name = "Webhooks",     description = "Inbound webhooks — M-Pesa STK callbacks"),
    ),
    servers(
        (url = "/", description = "Current server")
    )
)]
pub struct ApiDoc;
