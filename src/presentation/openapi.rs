use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // ── Health ────────────────────────────────────────────────────────────
        crate::presentation::http::handlers::health::health,
        crate::presentation::http::handlers::health::ready,

        // ── Auth ──────────────────────────────────────────────────────────────
        crate::presentation::http::handlers::auth::staff_login,
        crate::presentation::http::handlers::auth::get_me,
        crate::presentation::http::handlers::auth::accept_invite,
        crate::presentation::http::handlers::auth::change_password,
        crate::presentation::http::handlers::auth::refresh,

        // ── Agency Admin ──────────────────────────────────────────────────────
        crate::presentation::http::handlers::agency::create_agency,
        crate::presentation::http::handlers::agency::grant_permission_tuple,
        crate::presentation::http::handlers::agency::revoke_permission_tuple,
        crate::presentation::http::handlers::agency::publish_auth_model,
        crate::presentation::http::handlers::agency::create_staff_user,

          // ── Staff management ──────────────────────────────────────────────────
        crate::presentation::http::handlers::staff::invite_staff,
        crate::presentation::http::handlers::staff::list_staff,
        crate::presentation::http::handlers::staff::get_staff_member,
        crate::presentation::http::handlers::staff::deactivate_staff,
        

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
        crate::presentation::http::handlers::resident::get_my_profile,
        crate::presentation::http::handlers::resident::list_my_payments,

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
        crate::presentation::http::handlers::maintenance::list_work_order_comments,
        crate::presentation::http::handlers::maintenance::create_work_order_comment,
        crate::presentation::http::handlers::maintenance::list_comment_replies,
        crate::presentation::http::handlers::maintenance::get_work_order_activity,

        // ── Owners ────────────────────────────────────────────────────────────
        crate::presentation::http::handlers::owner::list_owners,
        crate::presentation::http::handlers::owner::get_owner,
        crate::presentation::http::handlers::owner::create_owner,
        crate::presentation::http::handlers::owner::invite_owner,
        crate::presentation::http::handlers::owner::update_owner,
        crate::presentation::http::handlers::owner::assign_owner_to_property,
        crate::presentation::http::handlers::owner::get_my_profile,
        crate::presentation::http::handlers::owner::list_my_properties,
        crate::presentation::http::handlers::owner::list_my_disbursements,

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

        // ── Documents ─────────────────────────────────────────────────────────
        crate::presentation::http::handlers::document::list_documents,
        crate::presentation::http::handlers::document::get_document,
        crate::presentation::http::handlers::document::upload_document,
        crate::presentation::http::handlers::document::delete_document,

        // ── Upload ────────────────────────────────────────────────────────────
        crate::presentation::http::handlers::upload::upload_file,

        // ── Inspections ───────────────────────────────────────────────────────
        crate::presentation::http::handlers::inspection::list_inspections,
        crate::presentation::http::handlers::inspection::get_inspection,
        crate::presentation::http::handlers::inspection::create_inspection,
        crate::presentation::http::handlers::inspection::update_inspection,
        crate::presentation::http::handlers::inspection::delete_inspection,

        // ── Invoices ──────────────────────────────────────────────────────────
        crate::presentation::http::handlers::invoice::list_invoices,
        crate::presentation::http::handlers::invoice::get_invoice,
        crate::presentation::http::handlers::invoice::create_invoice,
        crate::presentation::http::handlers::invoice::update_invoice_status,

        // ── Analytics ─────────────────────────────────────────────────────────
        crate::presentation::http::handlers::analytics::portfolio_analytics,
        crate::presentation::http::handlers::analytics::revenue_report,
        crate::presentation::http::handlers::analytics::occupancy_trends,

        // ── Insights ─────────────────────────────────────────────────────────
        crate::presentation::http::handlers::insights::rent_default_risk,
        crate::presentation::http::handlers::insights::tenant_churn,
        crate::presentation::http::handlers::insights::maintenance_alerts,
        crate::presentation::http::handlers::insights::expense_forecast,
        crate::presentation::http::handlers::insights::vendor_allocation,

        // ── Disbursements ─────────────────────────────────────────────────────
        crate::presentation::http::handlers::disbursement::list_disbursements,
        crate::presentation::http::handlers::disbursement::get_disbursement,
        crate::presentation::http::handlers::disbursement::create_disbursement,
        crate::presentation::http::handlers::disbursement::update_disbursement_status,
        crate::presentation::http::handlers::disbursement::initiate_payout,

        // ── Accounting — chart of accounts ────────────────────────────────────
        crate::presentation::http::handlers::accounting::list_accounts,
        crate::presentation::http::handlers::accounting::create_account,
        crate::presentation::http::handlers::accounting::delete_account,

        // ── Accounting — journal entries ──────────────────────────────────────
        crate::presentation::http::handlers::accounting::list_journal_entries,
        crate::presentation::http::handlers::accounting::get_journal_entry,
        crate::presentation::http::handlers::accounting::post_journal_entry,
        crate::presentation::http::handlers::accounting::void_journal_entry,

        // ── Accounting — reports ──────────────────────────────────────────────
        crate::presentation::http::handlers::accounting::get_trial_balance,
        crate::presentation::http::handlers::accounting::get_vat_report,

        // ── Bank reconciliation ───────────────────────────────────────────────
        crate::presentation::http::handlers::bank_reconciliation::list_statements,
        crate::presentation::http::handlers::bank_reconciliation::get_statement,
        crate::presentation::http::handlers::bank_reconciliation::import_statement,
        crate::presentation::http::handlers::bank_reconciliation::match_line,
        crate::presentation::http::handlers::bank_reconciliation::unmatch_line,
        crate::presentation::http::handlers::bank_reconciliation::get_reconciliation_report,

        // ── Subscription ──────────────────────────────────────────────────────
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
        crate::presentation::error::ErrorResponse,

        // ── Domain enums ──────────────────────────────────────────────────────
        crate::domain::enums::PropertyType,
        crate::domain::property::PropertyConfig,
        crate::domain::property::PropertyPolicies,
        crate::domain::property::PaymentMethod,
        crate::domain::property::PaymentMethodKind,
        crate::domain::property::ServiceCharges,
        crate::domain::property::PropertyDocument,
        crate::domain::property::BuildingClass,
        crate::domain::property::BillingCycle,
        crate::domain::enums::LedgerEntryType,
        crate::domain::enums::WorkOrderPriority,
        crate::domain::enums::WorkOrderStatus,
        crate::domain::enums::PaymentClaimStatus,
        crate::domain::enums::PaymentMethodType,
        crate::domain::enums::BillingFrequency,
        crate::domain::enums::AgreementStatus,
        crate::domain::enums::MeterType,
        crate::domain::enums::BillingMode,
        crate::domain::enums::UtilityBillStatus,
        crate::domain::enums::VendorStatus,
        crate::domain::enums::PortalStatus,
        crate::domain::enums::InspectionStatus,
        crate::domain::enums::InspectionType,

        // ── Accounting domain types ────────────────────────────────────────────
        crate::domain::accounting::AccountType,
        crate::domain::accounting::JournalEntryStatus,
        crate::domain::accounting::JournalLine,

        // ── Bank reconciliation domain types ──────────────────────────────────
        crate::domain::bank_reconciliation::ReconciliationStatus,

        // ── Auth ──────────────────────────────────────────────────────────────
        crate::presentation::http::dto::auth::LoginDto,
        crate::presentation::http::dto::auth::RegisterDto,
        crate::presentation::http::dto::auth::RefreshDto,
        crate::presentation::http::responses::auth::LoginResponse,
        crate::presentation::http::responses::auth::MeResponse,
        crate::presentation::http::responses::auth::TokenResponse,

        // ── Agency ────────────────────────────────────────────────────────────
        crate::presentation::http::dto::agency::CreateAgencyDto,
        crate::presentation::http::dto::openfga::WriteTupleDto,
        crate::presentation::http::dto::openfga::DeleteTupleDto,
        crate::presentation::http::dto::openfga::UpdateAuthModelDto,
        crate::presentation::http::responses::agency::AgencyResponse,
        crate::presentation::http::responses::openfga::ModelVersionResponse,

        // ── Staff ─────────────────────────────────────────────────────────────
        crate::presentation::http::dto::staff::CreateStaffUserDto,
        crate::presentation::http::dto::staff::InviteStaffDto,
        crate::presentation::http::dto::staff::ListStaffParams,
        crate::presentation::http::responses::staff::StaffUserResponse,
        crate::presentation::http::responses::staff::CreatedStaffUserResponse,
        crate::presentation::http::responses::staff::InviteStaffResponse,

        // ── Properties ────────────────────────────────────────────────────────
        crate::presentation::http::dto::property::CreatePropertyDto,
        crate::presentation::http::dto::property::UpdatePropertyDto,
        crate::presentation::http::dto::property::ListPropertiesParams,
        crate::presentation::http::responses::property::PropertyResponse,

        // ── Units ─────────────────────────────────────────────────────────────
        crate::presentation::http::dto::unit::CreateUnitDto,
        crate::presentation::http::dto::unit::UpdateUnitDto,
        crate::presentation::http::responses::unit::UnitResponse,

        // ── Residents ─────────────────────────────────────────────────────────
        crate::presentation::http::dto::resident::InviteResidentDto,
        crate::presentation::http::dto::resident::ListResidentsParams,
        crate::presentation::http::responses::resident::ResidentResponse,

        // ── Agreements ────────────────────────────────────────────────────────
        crate::presentation::http::dto::agreement::CreateAgreementDto,
        crate::presentation::http::dto::agreement::ListAgreementsParams,
        crate::presentation::http::responses::agreement::AgreementResponse,

        // ── Payments ──────────────────────────────────────────────────────────
        crate::presentation::http::dto::payment::SubmitClaimDto,
        crate::presentation::http::dto::payment::ReviewClaimDto,
        crate::presentation::http::dto::payment::ListClaimsParams,
        crate::presentation::http::responses::payment::PaymentClaimResponse,

        // ── Ledger ────────────────────────────────────────────────────────────
        crate::presentation::http::dto::ledger::PostChargeDto,
        crate::presentation::http::dto::ledger::ListLedgerParams,
        crate::presentation::http::responses::ledger::LedgerEntryResponse,
        crate::presentation::http::responses::ledger::BalanceSummaryResponse,

        // ── Maintenance ───────────────────────────────────────────────────────
        crate::presentation::http::dto::maintenance::CreateWorkOrderDto,
        crate::presentation::http::dto::maintenance::UpdateWorkOrderDto,
        crate::presentation::http::dto::maintenance::ListWorkOrdersParams,
        crate::presentation::http::dto::maintenance::CreateWorkOrderCommentDto,
        crate::presentation::http::responses::maintenance::WorkOrderResponse,
        crate::presentation::http::responses::maintenance::WorkOrderCommentResponse,
        crate::presentation::http::responses::maintenance::WorkOrderSubtaskResponse,
        crate::presentation::http::responses::maintenance::WorkOrderAttachmentResponse,
        crate::presentation::http::responses::maintenance::WorkOrderActivityResponse,

        // ── Owners ────────────────────────────────────────────────────────────
        crate::presentation::http::dto::owner::CreateOwnerDto,
        crate::presentation::http::dto::owner::UpdateOwnerDto,
        crate::presentation::http::dto::owner::AssignOwnerDto,
        crate::presentation::http::dto::owner::ListOwnersParams,
        crate::presentation::http::responses::owner::OwnerResponse,
        crate::presentation::http::responses::owner::DisbursementResponse,
        crate::presentation::http::responses::property::PropertyWithPercentResponse,

        // ── Vendors ───────────────────────────────────────────────────────────
        crate::presentation::http::dto::vendor::CreateVendorDto,
        crate::presentation::http::dto::vendor::UpdateVendorDto,
        crate::presentation::http::dto::vendor::ListVendorsParams,
        crate::presentation::http::responses::vendor::VendorResponse,

        // ── Utility ───────────────────────────────────────────────────────────
        crate::presentation::http::dto::utility::CreateMeterDto,
        crate::presentation::http::dto::utility::RecordReadingDto,
        crate::presentation::http::responses::utility::UtilityMeterResponse,
        crate::presentation::http::responses::utility::MeterReadingResponse,
        crate::presentation::http::responses::utility::UtilityBillResponse,

        // ── Documents ─────────────────────────────────────────────────────────
        // NOTE: ListDocumentsParams uses IntoParams (query string), not ToSchema.
        // It must NOT appear here — only in the handler's params() annotation.
        crate::presentation::http::responses::document::DocumentResponse,
        crate::presentation::http::responses::upload::UploadResponse,

        // ── Inspections ───────────────────────────────────────────────────────
        crate::presentation::http::dto::inspection::CreateInspectionDto,
        crate::presentation::http::dto::inspection::UpdateInspectionDto,
        crate::presentation::http::responses::inspection::InspectionResponse,

        // ── Invoices ──────────────────────────────────────────────────────────
        crate::presentation::http::dto::invoice::CreateInvoiceDto,
        crate::presentation::http::dto::invoice::UpdateInvoiceStatusDto,
        crate::presentation::http::responses::invoice::InvoiceResponse,

        // ── Disbursements ─────────────────────────────────────────────────────
        crate::presentation::http::dto::disbursement::CreateDisbursementDto,
        crate::presentation::http::dto::disbursement::UpdateDisbursementStatusDto,
        crate::presentation::http::responses::disbursement::DisbursementResponse,

        // ── Accounting ────────────────────────────────────────────────────────
        crate::presentation::http::dto::accounting::CreateAccountDto,
        crate::presentation::http::dto::accounting::PostJournalEntryDto,
        crate::presentation::http::responses::accounting::AccountResponse,
        crate::presentation::http::responses::accounting::JournalLineResponse,
        crate::presentation::http::responses::accounting::JournalEntryResponse,
        crate::presentation::http::responses::accounting::TrialBalanceLineResponse,
        crate::presentation::http::responses::accounting::TrialBalanceResponse,
        crate::presentation::http::responses::accounting::VatReportLineResponse,
        crate::presentation::http::responses::accounting::VatReportResponse,

        // ── Bank reconciliation ───────────────────────────────────────────────
        crate::presentation::http::dto::bank_reconciliation::ImportStatementDto,
        crate::presentation::http::dto::bank_reconciliation::ImportStatementLineDto,
        crate::presentation::http::dto::bank_reconciliation::MatchLineDto,
        crate::presentation::http::responses::bank_reconciliation::BankStatementResponse,
        crate::presentation::http::responses::bank_reconciliation::BankStatementLineResponse,
        crate::presentation::http::responses::bank_reconciliation::ReconciliationReportResponse,

        // ── Subscription ──────────────────────────────────────────────────────
        crate::presentation::http::responses::subscription::PlanResponse,
        crate::presentation::http::responses::subscription::SubscriptionStatusResponse,
        crate::presentation::http::dto::subscription::ChangePlanDto,
        crate::presentation::http::dto::subscription::CancelDto,
        crate::presentation::http::dto::subscription::SetOverrideDto,
        crate::presentation::http::dto::subscription::InitiatePaymentDto,
        crate::presentation::http::responses::subscription::InitiatePaymentResponse,

        // ── Webhooks ──────────────────────────────────────────────────────────
        crate::presentation::http::dto::webhook::MpesaCallback,
        crate::presentation::http::dto::webhook::MpesaCallbackBody,
        crate::presentation::http::dto::webhook::StkCallback,
        crate::presentation::http::dto::webhook::CallbackMetadata,
        crate::presentation::http::dto::webhook::CallbackItem,

        // ── Health ────────────────────────────────────────────────────────────
        crate::presentation::http::responses::health::HealthResponse,
        crate::presentation::http::responses::health::ReadyResponse,
        crate::presentation::http::responses::health::ComponentStatus,
        crate::presentation::http::responses::health::DependencyStatus,
        crate::presentation::http::responses::health::DatabasePoolStatus,
        crate::presentation::http::responses::health::RuntimeStatus,
        crate::presentation::http::responses::health::ServiceInfo,
        crate::presentation::http::responses::health::ConfigurationStatus,
    )),
    tags(
        (name = "Health",              description = "Liveness and readiness probes"),
        (name = "Auth",                description = "JWT authentication"),
        (name = "Properties",          description = "Property CRUD"),
        (name = "Residents",           description = "Resident management"),
        (name = "Agreements",          description = "Lease agreements"),
        (name = "Payments",            description = "Payment claim submission and review"),
        (name = "Ledger",              description = "Resident-facing financial ledger"),
        (name = "Maintenance",         description = "Work orders"),
        (name = "Owners",              description = "Owner portal"),
        (name = "Vendors",             description = "Vendor management"),
        (name = "Utility",             description = "Utility meters, readings and bills"),
        (name = "Documents",           description = "Persisted tenant-owned documents"),
        (name = "Upload",              description = "Ephemeral file attachments"),
        (name = "Inspections",         description = "Property and unit inspections"),
        (name = "Invoices",            description = "Tenant invoices"),
        (name = "Analytics",           description = "Portfolio analytics (Growth+)"),
        (name = "Insights",            description = "Predictive analytics (Growth+)"),
        (name = "Disbursements",       description = "Owner disbursements"),
        (name = "Accounting",          description = "Double-entry accounting, VAT, trial balance (Growth+)"),
        (name = "BankReconciliation",  description = "Bank statement import and reconciliation (Growth+)"),
        (name = "Subscription",        description = "Subscription plans and billing"),
        (name = "Admin",               description = "Platform-admin subscription management"),
        (name = "Webhooks",            description = "Inbound webhooks — M-Pesa STK callbacks"),
    ),
    servers(
        (url = "/", description = "Current server")
    )
)]
pub struct ApiDoc;
