//
// Document management domain. Each document is a file stored in S3 and
// catalogued in the `documents` table with metadata and entity linkage.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

// ── Entity ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Document {
    pub id: Uuid,
    pub agency_id: Uuid,

    // S3
    pub s3_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,

    // Classification
    pub document_type: DocumentType,
    pub title: Option<String>,
    pub notes: Option<String>,

    // Entity linkage (all optional — a document can attach to any one entity)
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub work_order_id: Option<Uuid>,

    pub uploaded_by: Uuid,
    pub created_at: OffsetDateTime,
}

// ── Classification enum ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    // Legal & Contract Documents
    LeaseAgreement,
    RentalAgreement,
    TenancyAgreement,
    SubleaseAgreement,
    Addendum,
    Amendment,
    TerminationNotice,
    EvictionNotice,

    // Certificates & Approvals
    Noc, // No-Objection Certificate
    OccupancyCertificate,
    CompletionCertificate,
    FireSafetyCertificate,
    StructuralStabilityCertificate,

    // Inspection & Assessment
    InspectionForm,
    InspectionReport,
    ConditionReport,
    MoveInChecklist,
    MoveOutChecklist,
    MaintenanceInspection,
    SafetyInspection,

    // Financial Documents
    UtilityBill,
    Receipt,
    Invoice,
    PaymentProof,
    DepositReceipt,
    RentReceipt,
    SecurityDepositRecord,
    LedgerStatement,

    // Identification & Verification
    IdDocument,
    Passport,
    NationalId,
    DriverLicense,
    ProofOfAddress,
    EmploymentLetter,
    BankStatement,
    CreditReport,

    // Property Documents
    TitleDeed,
    SurveyPlan,
    SitePlan,
    FloorPlan,
    ArchitecturalDrawing,
    StructuralDrawing,
    BuildingPermit,
    RenovationApproval,

    // Communication & Notices
    NoticeToTenant,
    NoticeToOwner,
    WarningLetter,
    ComplaintForm,
    FeedbackForm,

    // Operational Documents
    WorkOrder,
    MaintenanceRequest,
    ServiceReport,
    VendorContract,
    ServiceAgreement,

    // Media & Visuals
    Photo,
    Video,
    Panorama,
    Blueprint,

    // Miscellaneous
    InsurancePolicy,
    WarrantyDocument,
    Manual,
    Guide,
    Checklist,
    Form,
    Template,
    Other,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            // Legal & Contract Documents
            Self::LeaseAgreement => "lease_agreement",
            Self::RentalAgreement => "rental_agreement",
            Self::TenancyAgreement => "tenancy_agreement",
            Self::SubleaseAgreement => "sublease_agreement",
            Self::Addendum => "addendum",
            Self::Amendment => "amendment",
            Self::TerminationNotice => "termination_notice",
            Self::EvictionNotice => "eviction_notice",

            // Certificates & Approvals
            Self::Noc => "noc",
            Self::OccupancyCertificate => "occupancy_certificate",
            Self::CompletionCertificate => "completion_certificate",
            Self::FireSafetyCertificate => "fire_safety_certificate",
            Self::StructuralStabilityCertificate => "structural_stability_certificate",

            // Inspection & Assessment
            Self::InspectionForm => "inspection_form",
            Self::InspectionReport => "inspection_report",
            Self::ConditionReport => "condition_report",
            Self::MoveInChecklist => "move_in_checklist",
            Self::MoveOutChecklist => "move_out_checklist",
            Self::MaintenanceInspection => "maintenance_inspection",
            Self::SafetyInspection => "safety_inspection",

            // Financial Documents
            Self::UtilityBill => "utility_bill",
            Self::Receipt => "receipt",
            Self::Invoice => "invoice",
            Self::PaymentProof => "payment_proof",
            Self::DepositReceipt => "deposit_receipt",
            Self::RentReceipt => "rent_receipt",
            Self::SecurityDepositRecord => "security_deposit_record",
            Self::LedgerStatement => "ledger_statement",

            // Identification & Verification
            Self::IdDocument => "id_document",
            Self::Passport => "passport",
            Self::NationalId => "national_id",
            Self::DriverLicense => "driver_license",
            Self::ProofOfAddress => "proof_of_address",
            Self::EmploymentLetter => "employment_letter",
            Self::BankStatement => "bank_statement",
            Self::CreditReport => "credit_report",

            // Property Documents
            Self::TitleDeed => "title_deed",
            Self::SurveyPlan => "survey_plan",
            Self::SitePlan => "site_plan",
            Self::FloorPlan => "floor_plan",
            Self::ArchitecturalDrawing => "architectural_drawing",
            Self::StructuralDrawing => "structural_drawing",
            Self::BuildingPermit => "building_permit",
            Self::RenovationApproval => "renovation_approval",

            // Communication & Notices
            Self::NoticeToTenant => "notice_to_tenant",
            Self::NoticeToOwner => "notice_to_owner",
            Self::WarningLetter => "warning_letter",
            Self::ComplaintForm => "complaint_form",
            Self::FeedbackForm => "feedback_form",

            // Operational Documents
            Self::WorkOrder => "work_order",
            Self::MaintenanceRequest => "maintenance_request",
            Self::ServiceReport => "service_report",
            Self::VendorContract => "vendor_contract",
            Self::ServiceAgreement => "service_agreement",

            // Media & Visuals
            Self::Photo => "photo",
            Self::Video => "video",
            Self::Panorama => "panorama",
            Self::Blueprint => "blueprint",

            // Miscellaneous
            Self::InsurancePolicy => "insurance_policy",
            Self::WarrantyDocument => "warranty_document",
            Self::Manual => "manual",
            Self::Guide => "guide",
            Self::Checklist => "checklist",
            Self::Form => "form",
            Self::Template => "template",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // Legal & Contract Documents
            "lease_agreement" => Some(Self::LeaseAgreement),
            "rental_agreement" => Some(Self::RentalAgreement),
            "tenancy_agreement" => Some(Self::TenancyAgreement),
            "sublease_agreement" => Some(Self::SubleaseAgreement),
            "addendum" => Some(Self::Addendum),
            "amendment" => Some(Self::Amendment),
            "termination_notice" => Some(Self::TerminationNotice),
            "eviction_notice" => Some(Self::EvictionNotice),

            // Certificates & Approvals
            "noc" => Some(Self::Noc),
            "occupancy_certificate" => Some(Self::OccupancyCertificate),
            "completion_certificate" => Some(Self::CompletionCertificate),
            "fire_safety_certificate" => Some(Self::FireSafetyCertificate),
            "structural_stability_certificate" => Some(Self::StructuralStabilityCertificate),

            // Inspection & Assessment
            "inspection_form" => Some(Self::InspectionForm),
            "inspection_report" => Some(Self::InspectionReport),
            "condition_report" => Some(Self::ConditionReport),
            "move_in_checklist" => Some(Self::MoveInChecklist),
            "move_out_checklist" => Some(Self::MoveOutChecklist),
            "maintenance_inspection" => Some(Self::MaintenanceInspection),
            "safety_inspection" => Some(Self::SafetyInspection),

            // Financial Documents
            "utility_bill" => Some(Self::UtilityBill),
            "receipt" => Some(Self::Receipt),
            "invoice" => Some(Self::Invoice),
            "payment_proof" => Some(Self::PaymentProof),
            "deposit_receipt" => Some(Self::DepositReceipt),
            "rent_receipt" => Some(Self::RentReceipt),
            "security_deposit_record" => Some(Self::SecurityDepositRecord),
            "ledger_statement" => Some(Self::LedgerStatement),

            // Identification & Verification
            "id_document" => Some(Self::IdDocument),
            "passport" => Some(Self::Passport),
            "national_id" => Some(Self::NationalId),
            "driver_license" => Some(Self::DriverLicense),
            "proof_of_address" => Some(Self::ProofOfAddress),
            "employment_letter" => Some(Self::EmploymentLetter),
            "bank_statement" => Some(Self::BankStatement),
            "credit_report" => Some(Self::CreditReport),

            // Property Documents
            "title_deed" => Some(Self::TitleDeed),
            "survey_plan" => Some(Self::SurveyPlan),
            "site_plan" => Some(Self::SitePlan),
            "floor_plan" => Some(Self::FloorPlan),
            "architectural_drawing" => Some(Self::ArchitecturalDrawing),
            "structural_drawing" => Some(Self::StructuralDrawing),
            "building_permit" => Some(Self::BuildingPermit),
            "renovation_approval" => Some(Self::RenovationApproval),

            // Communication & Notices
            "notice_to_tenant" => Some(Self::NoticeToTenant),
            "notice_to_owner" => Some(Self::NoticeToOwner),
            "warning_letter" => Some(Self::WarningLetter),
            "complaint_form" => Some(Self::ComplaintForm),
            "feedback_form" => Some(Self::FeedbackForm),

            // Operational Documents
            "work_order" => Some(Self::WorkOrder),
            "maintenance_request" => Some(Self::MaintenanceRequest),
            "service_report" => Some(Self::ServiceReport),
            "vendor_contract" => Some(Self::VendorContract),
            "service_agreement" => Some(Self::ServiceAgreement),

            // Media & Visuals
            "photo" => Some(Self::Photo),
            "video" => Some(Self::Video),
            "panorama" => Some(Self::Panorama),
            "blueprint" => Some(Self::Blueprint),

            // Miscellaneous
            "insurance_policy" => Some(Self::InsurancePolicy),
            "warranty_document" => Some(Self::WarrantyDocument),
            "manual" => Some(Self::Manual),
            "guide" => Some(Self::Guide),
            "checklist" => Some(Self::Checklist),
            "form" => Some(Self::Form),
            "template" => Some(Self::Template),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

pub struct CreateDocumentCommand {
    pub agency_id: Uuid,
    pub s3_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub document_type: DocumentType,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub work_order_id: Option<Uuid>,
    pub uploaded_by: Uuid,
}

pub struct DocumentFilter {
    pub agency_id: Uuid,
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub work_order_id: Option<Uuid>,
    pub document_type: Option<DocumentType>,
    pub limit: i64,
    pub offset: i64,
}
