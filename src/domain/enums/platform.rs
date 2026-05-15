// PLATFORM DB ENUMS
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// agencies.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "agency_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AgencyStatus {
    Active,
    Suspended,
    Deprovisioned,
}

/// users / user_agency_roles.role  +  invite_tokens.role  +  refresh_tokens.role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    PlatformAdmin,
    Admin,
    Manager,
    Agent,
    Resident,
    Owner,
    Vendor,
}

/// portal_user_index.portal  +  invite_tokens.portal  +  refresh_tokens.portal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "portal_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PortalType {
    Staff,
    Resident,
    Owner,
    Vendor,
    Caretaker,
}

/// portal_user_index.contact_type  +  invite_tokens.contact_type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "contact_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ContactType {
    Email,
    Phone,
}

/// subscriptions.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "subscription_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Cancelled,
}

/// subscription_invoices.status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "invoice_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionInvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

/// subscription_plans.billing_interval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "billing_interval", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BillingInterval {
    Monthly,
    Yearly,
}
