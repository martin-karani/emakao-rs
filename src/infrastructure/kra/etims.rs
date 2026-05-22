// src/infrastructure/kra/etims.rs
//
// eTIMS (Electronic Tax Invoice Management System) client.
//
// Sandbox: https://etims-sbx.kra.go.ke
// Production: https://etims.kra.go.ke
//
// The eTIMS integration requires a KRA-certified On-Site Control Unit (OSCU)
// or Virtual Sales Control Unit (VSCU) per business location.  The certification
// process involves:
//   1. Registration on the eTIMS portal with agency KRA PIN.
//   2. Device serial number assignment.
//   3. Invoice signing with KRA private key exchange.
//   4. QR code embedding on every printed/digital invoice.
//
// This client is STUBBED until the agency completes eTIMS certification.
// All methods return EtimsNotEnabled when the feature flag is off.

use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::tax::EtimsInvoiceRef;

#[derive(Clone, Debug)]
pub struct EtimsConfig {
    pub base_url: String, // "https://etims-sbx.kra.go.ke" or production
    pub device_serial: String,
    pub agency_kra_pin: String,
    pub enabled: bool,
}

impl EtimsConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            base_url: std::env::var("KRA_ETIMS_BASE_URL")
                .unwrap_or_else(|_| "https://etims-sbx.kra.go.ke".into()),
            device_serial: std::env::var("KRA_ETIMS_DEVICE_SERIAL")
                .unwrap_or_else(|_| "EMAKAO-DEV-001".into()),
            agency_kra_pin: std::env::var("KRA_ETIMS_AGENCY_PIN").unwrap_or_default(),
            enabled: std::env::var("KRA_ETIMS_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(false),
        })
    }
}

#[derive(Debug)]
pub struct EtimsInvoicePayload {
    pub invoice_number: String,
    pub invoice_date: OffsetDateTime,
    pub buyer_kra_pin: Option<String>,
    pub buyer_name: String,
    pub line_items: Vec<EtimsLineItem>,
    pub total_excl_vat: rust_decimal::Decimal,
    pub vat_amount: rust_decimal::Decimal,
    pub total_incl_vat: rust_decimal::Decimal,
    /// Internal reference for idempotency.
    pub internal_ref: Uuid,
}

#[derive(Debug)]
pub struct EtimsLineItem {
    pub description: String,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    pub vat_class: EtimsVatClass,
    pub total: rust_decimal::Decimal,
}

/// KRA eTIMS VAT classification codes.
#[derive(Debug, Clone, Copy)]
pub enum EtimsVatClass {
    /// Standard-rated 16 %.
    A,
    /// Zero-rated exports.
    B,
    /// Zero-rated (other).
    C,
    /// Exempt.
    E,
}

pub struct EtimsClient {
    config: EtimsConfig,
    http: reqwest::Client,
}

impl EtimsClient {
    pub fn new(config: EtimsConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap();
        Self { config, http }
    }

    /// Submit an invoice to eTIMS and receive a CU invoice number + QR code.
    ///
    /// Returns `None` when eTIMS is disabled (agency not yet certified).
    pub async fn submit_invoice(
        &self,
        payload: EtimsInvoicePayload,
    ) -> anyhow::Result<Option<EtimsInvoiceRef>> {
        if !self.config.enabled {
            tracing::debug!(
                invoice_number = %payload.invoice_number,
                "eTIMS disabled — invoice will not be fiscalised until certification"
            );
            return Ok(None);
        }

        // TODO: implement real eTIMS VSCU API call once certified.
        // The sandbox endpoint structure is:
        //   POST {base_url}/vscu/submitInvoice
        //   Headers: Content-Type: application/json, deviceSerialNo: {serial}
        //   Body: KRA-specified JSON schema (RA invoice object)

        tracing::warn!(
            invoice_number = %payload.invoice_number,
            "eTIMS submit_invoice: sandbox call not yet implemented"
        );

        // Return a synthetic sandbox response for development testing
        Ok(Some(EtimsInvoiceRef {
            cu_invoice_number: format!("SBX-{}", payload.invoice_number),
            qr_code: format!(
                "https://etims-sbx.kra.go.ke/verify?ref=SBX-{}",
                payload.invoice_number
            ),
            accepted_at: OffsetDateTime::now_utc(),
            device_serial: self.config.device_serial.clone(),
        }))
    }
}
