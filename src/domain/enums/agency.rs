use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// AGENCY DB ENUMS

/// properties.property_type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "property_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Residential,
    Multifamily,
    Commercial,
    Community,
    Student,
    AffordableHousing,
    Affordable,
}

/// units.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "unit_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UnitStatus {
    Vacant,
    Occupied,
    Maintenance,
    Reserved,
    Inactive,
}

/// residents.portal_status  |  owners.portal_status  |  vendors.portal_status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "portal_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PortalStatus {
    Invited,
    Active,
    Suspended,
}

/// agreements.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "agreement_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AgreementStatus {
    Draft,
    PendingSignature,
    Active,
    Expired,
    Terminated,
    PendingRenewal,
    Renewed,
}

/// agreements.billing_frequency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "billing_frequency", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BillingFrequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    OneTime,
}

/// payment_claims.method_type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "payment_method_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodType {
    MpesaPaybill,
    MpesaTill,
    BankTransfer,
    Cash,
}

/// payment_claims.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "payment_claim_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PaymentClaimStatus {
    PendingReview,
    Approved,
    Rejected,
}

/// ledger_entries.entry_type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "ledger_entry_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum LedgerEntryType {
    Rent,
    Deposit,
    HoaDues,
    CamCharge,
    Utility,
    MaintenanceCharge,
    LateFee,
    LegalFee,
    Penalty,
    PaymentMpesa,
    PaymentBank,
    PaymentCash,
    DepositRefund,
    CreditNote,
    Waiver,
    Disbursement,
    JournalAdjustment,
}

impl LedgerEntryType {
    pub fn is_payment(&self) -> bool {
        matches!(
            self,
            Self::PaymentMpesa | Self::PaymentBank | Self::PaymentCash
        )
    }

    pub fn is_charge(&self) -> bool {
        matches!(
            self,
            Self::Rent
                | Self::Deposit
                | Self::HoaDues
                | Self::CamCharge
                | Self::Utility
                | Self::MaintenanceCharge
                | Self::LateFee
                | Self::LegalFee
                | Self::Penalty
        )
    }

    pub fn from_payment_method(method: PaymentMethodType) -> Self {
        match method {
            PaymentMethodType::MpesaPaybill | PaymentMethodType::MpesaTill => Self::PaymentMpesa,
            PaymentMethodType::BankTransfer => Self::PaymentBank,
            PaymentMethodType::Cash => Self::PaymentCash,
        }
    }
}

/// work_orders.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "work_order_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum WorkOrderStatus {
    Open,
    InProgress,
    Completed,
    Cancelled,
}

/// work_orders.priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "work_order_priority", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum WorkOrderPriority {
    Low,
    Medium,
    High,
    Emergency,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, sqlx::Type, ToSchema)]
#[sqlx(type_name = "work_order_category", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum WorkOrderCategory {
    Plumbing,
    Electrical,
    Structural,
    Hvac,
    Appliance,
    Painting,
    Cleaning,
    Security,
    Landscaping,
    PestControl,
    General,
}

impl WorkOrderCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Plumbing => "plumbing",
            Self::Electrical => "electrical",
            Self::Structural => "structural",
            Self::Hvac => "hvac",
            Self::Appliance => "appliance",
            Self::Painting => "painting",
            Self::Cleaning => "cleaning",
            Self::Security => "security",
            Self::Landscaping => "landscaping",
            Self::PestControl => "pestcontrol",
            Self::General => "general",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, sqlx::Type, ToSchema)]
#[sqlx(type_name = "work_order_reporter_type", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum WorkOrderReporterType {
    Staff,
    Resident,
    Caretaker,
    Owner,
    Vendor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, sqlx::Type, ToSchema)]
#[sqlx(type_name = "work_order_comment_author_type", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum WorkOrderCommentAuthorType {
    Staff,
    Resident,
    Caretaker,
    Owner,
    Vendor,
}

/// vendors.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "vendor_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum VendorStatus {
    Active,
    Inactive,
    Blacklisted,
}

/// utility_meters.meter_type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "meter_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MeterType {
    Electricity,
    Water,
    Gas,
}

/// utility_meters.billing_mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "billing_mode", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BillingMode {
    Prepaid,
    Postpaid,
}

/// utility_bills.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "utility_bill_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UtilityBillStatus {
    Draft,
    Issued,
    Paid,
    Overdue,
}

/// applicants.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "applicant_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ApplicantStatus {
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Withdrawn,
}

/// disbursements.method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "disbursement_method", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum DisbursementMethod {
    BankTransfer,
    MpesaB2c,
    Cheque,
}

/// disbursements.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "disbursement_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum DisbursementStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// inspections.inspection_type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "inspection_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InspectionType {
    MoveIn,
    MoveOut,
    Routine,
    Emergency,
}

/// inspections.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "inspection_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InspectionStatus {
    Scheduled,
    InProgress,
    Completed,
    Cancelled,
}

/// messages.sender_type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "sender_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SenderType {
    Staff,
    Resident,
    Owner,
    Vendor,
    System,
}

/// invoices.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "invoice_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Paid,
    Overdue,
    Void,
}
