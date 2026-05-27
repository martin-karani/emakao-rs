// ─────────────────────────────────────────────────────────────────────────────
// src/domain/tax.rs
//
// Kenya-specific tax domain model for Emakao property management.
//
// Taxes covered:
//   • MRI  — Monthly Rental Income tax (7.5 % of gross rent, Finance Act 2023)
//   • VAT  — Value Added Tax on agency services (16 %)
//   • WHT  — Withholding Tax on management / professional fees (5 %)
//
// KRA thresholds (as at 1 Jan 2024):
//   • Annual gross rent < KES 288 000      → MRI exempt
//   • Annual gross rent KES 288 001–15 M   → MRI at 7.5 % (final tax)
//   • Annual gross rent > KES 15 M         → Normal income-tax regime
//   • Agency management fees               → VAT 16 % + WHT 5 % deducted by owner
// ─────────────────────────────────────────────────────────────────────────────

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use time::{Date, Month, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Constants ─────────────────────────────────────────────────────────────────

/// MRI rate effective 1 January 2024 (Finance Act 2023 reduced it from 10 %).
pub const MRI_RATE: Decimal = Decimal::from_parts(75, 0, 0, false, 3); // 0.075

/// Standard VAT rate (Kenya VAT Act, Cap 476).
pub const VAT_RATE: Decimal = Decimal::from_parts(16, 0, 0, false, 2); // 0.16

/// Withholding-tax rate on management / agency fees paid to residents.
pub const WHT_MANAGEMENT_FEE_RATE: Decimal = Decimal::from_parts(5, 0, 0, false, 2); // 0.05

/// Minimum annual gross rent subject to MRI (Finance Act 2020).
pub const MRI_ANNUAL_LOWER_THRESHOLD_KES: Decimal = Decimal::from_parts(288_000, 0, 0, false, 0);

/// Annual gross rent above which MRI no longer applies; normal income tax kicks in.
pub const MRI_ANNUAL_UPPER_THRESHOLD_KES: Decimal = Decimal::from_parts(15_000_000, 0, 0, false, 0);

// ── Enums ─────────────────────────────────────────────────────────────────────

/// Which KRA tax head an obligation belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaxObligationType {
    /// Monthly Rental Income tax — 7.5 % on gross rent.
    Mri,
    /// Value-added tax — 16 % on taxable services.
    Vat,
    /// Withholding tax — 5 % deducted by payer on management/professional fees.
    Wht,
}

impl fmt::Display for TaxObligationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mri => write!(f, "MRI"),
            Self::Vat => write!(f, "VAT"),
            Self::Wht => write!(f, "WHT"),
        }
    }
}

/// Filing / remittance lifecycle of a tax obligation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaxObligationStatus {
    /// Computed but not yet filed with KRA.
    Pending,
    /// Return submitted to KRA (iTax / eRITS); awaiting payment.
    Filed,
    /// Payment confirmed; PRN used and receipt obtained.
    Paid,
    /// Overdue — past due-date and not yet paid.
    Overdue,
    /// Nil return filed (no rent received in the period).
    NilFiled,
}

/// Which MRI tax regime applies to a given owner / property combination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MriRegime {
    /// Annual rent < KES 288 000 — exempt from MRI.
    Exempt,
    /// Annual rent KES 288 001–15 M — MRI at 7.5 % (final tax).
    Mri,
    /// Annual rent > KES 15 M — must file under normal income-tax regime.
    NormalIncomeTax,
    /// Owner has elected in writing to be taxed under the normal regime
    /// even though rent falls in the MRI band.
    ElectedNormal,
}

// ── Value objects ─────────────────────────────────────────────────────────────

/// A calendar year + month combination used as a tax period key.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
pub struct TaxPeriod {
    pub year: i32,
    pub month: u8, // 1-12
}

impl TaxPeriod {
    pub fn new(year: i32, month: u8) -> Self {
        assert!((1..=12).contains(&month), "month must be 1–12");
        Self { year, month }
    }

    /// Returns the period for the given date.
    pub fn from_date(d: Date) -> Self {
        Self {
            year: d.year(),
            month: d.month() as u8,
        }
    }

    /// Returns the period for the current UTC month.
    pub fn current() -> Self {
        Self::from_date(OffsetDateTime::now_utc().date())
    }

    /// The last day by which MRI / VAT returns must be filed
    /// (20th of the month following the tax period).
    pub fn mri_due_date(&self) -> Date {
        let (year, month) = if self.month == 12 {
            (self.year + 1, 1u8)
        } else {
            (self.year, self.month + 1)
        };
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), 20).unwrap()
    }

    /// Due date for WHT agents collecting rental income:
    /// 5th working day after the deduction was made.  We approximate
    /// this as the 5th calendar day of the month following the period.
    pub fn wht_agent_due_date(&self) -> Date {
        let (year, month) = if self.month == 12 {
            (self.year + 1, 1u8)
        } else {
            (self.year, self.month + 1)
        };
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), 5).unwrap()
    }

    /// Human-readable label, e.g. "2025-04".
    pub fn label(&self) -> String {
        format!("{}-{:02}", self.year, self.month)
    }

    /// Returns the period immediately preceding this one.
    pub fn previous(&self) -> Self {
        if self.month == 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                year: self.year,
                month: self.month - 1,
            }
        }
    }
}

impl fmt::Display for TaxPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Tax breakdown ─────────────────────────────────────────────────────────────

/// Replaces the old single `tax_kes` field on Invoice / LedgerEntry.
///
/// Rules:
///   • `mri_kes`  = 7.5 % × gross_rent_kes   (if MRI regime applies)
///   • `vat_kes`  = 16 % × management_fee_kes (if agency is VAT-registered)
///   • `wht_kes`  = 5  % × management_fee_kes (deducted by owner)
///   • `total_tax_kes` = mri_kes + vat_kes
///   • `net_payable_kes` = subtotal + vat_kes − wht_kes
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct TaxBreakdown {
    /// MRI component — zero when exempt or not a rent invoice.
    pub mri_kes: Decimal,
    /// Output VAT component — zero when not a VAT-applicable service.
    pub vat_kes: Decimal,
    /// WHT deducted by the owner — reduces cash received by agency.
    pub wht_kes: Decimal,
    /// mri_kes + vat_kes
    pub total_tax_kes: Decimal,
    /// Amount the owner actually transfers: subtotal + vat_kes − wht_kes
    pub net_payable_kes: Decimal,
    /// MRI regime determined for this period.
    pub mri_regime: Option<MriRegime>,
    /// Tax period this breakdown applies to.
    pub tax_period: Option<TaxPeriod>,
    /// Filing due date (20th of following month for MRI/VAT).
    pub due_date: Option<Date>,
}

impl TaxBreakdown {
    /// Compute the full breakdown for a rental invoice line.
    ///
    /// # Arguments
    /// * `gross_rent_kes`        — rent amount charged to the tenant
    /// * `annual_projection_kes` — estimated or YTD gross rent for the owner
    ///                             (used to determine which MRI regime applies)
    /// * `management_fee_kes`    — agency fee charged to the owner
    /// * `vat_registered`        — whether the agency is VAT-registered with KRA
    /// * `wht_agent`             — whether the agency is a KRA-appointed WHT agent
    /// * `tax_period`            — the calendar month being billed
    pub fn compute(
        gross_rent_kes: Decimal,
        annual_projection_kes: Decimal,
        management_fee_kes: Decimal,
        vat_registered: bool,
        wht_agent: bool,
        tax_period: TaxPeriod,
    ) -> Self {
        let zero = Decimal::ZERO;

        // 1. Determine MRI regime
        let mri_regime = if annual_projection_kes < MRI_ANNUAL_LOWER_THRESHOLD_KES {
            MriRegime::Exempt
        } else if annual_projection_kes > MRI_ANNUAL_UPPER_THRESHOLD_KES {
            MriRegime::NormalIncomeTax
        } else {
            MriRegime::Mri
        };

        // 2. Compute MRI (only if agency is WHT agent and regime applies)
        let mri_kes = if wht_agent && mri_regime == MriRegime::Mri {
            (gross_rent_kes * MRI_RATE).round_dp(2)
        } else {
            zero
        };

        // 3. Compute VAT on management fee
        let vat_kes = if vat_registered {
            (management_fee_kes * VAT_RATE).round_dp(2)
        } else {
            zero
        };

        // 4. Compute WHT on gross management fee (deducted by owner)
        let wht_kes = (management_fee_kes * WHT_MANAGEMENT_FEE_RATE).round_dp(2);

        let total_tax_kes = mri_kes + vat_kes;
        let net_payable_kes = management_fee_kes + vat_kes - wht_kes;

        let due_date = Some(tax_period.mri_due_date());

        Self {
            mri_kes,
            vat_kes,
            wht_kes,
            total_tax_kes,
            net_payable_kes,
            mri_regime: Some(mri_regime),
            tax_period: Some(tax_period),
            due_date,
        }
    }

    /// Returns a zero breakdown (no tax applicable).
    pub fn zero(tax_period: TaxPeriod) -> Self {
        Self {
            tax_period: Some(tax_period),
            due_date: Some(tax_period.mri_due_date()),
            ..Default::default()
        }
    }

    /// Returns true if any tax is due in this breakdown.
    pub fn has_tax(&self) -> bool {
        self.mri_kes > Decimal::ZERO || self.vat_kes > Decimal::ZERO || self.wht_kes > Decimal::ZERO
    }

    /// Late-filing penalty for MRI / VAT (whichever is higher: 5 % of tax or KES 2 000).
    pub fn late_filing_penalty_individual(&self) -> Decimal {
        let pct = (self.total_tax_kes * Decimal::new(5, 2)).round_dp(2);
        pct.max(Decimal::from(2_000))
    }

    /// Late-filing penalty for body corporates (5 % or KES 20 000).
    pub fn late_filing_penalty_corporate(&self) -> Decimal {
        let pct = (self.total_tax_kes * Decimal::new(5, 2)).round_dp(2);
        pct.max(Decimal::from(20_000))
    }

    /// Monthly interest on unpaid tax (1 % per month or part thereof).
    pub fn monthly_interest(&self) -> Decimal {
        (self.total_tax_kes * Decimal::new(1, 2)).round_dp(2)
    }
}

// ── Domain entities ───────────────────────────────────────────────────────────

/// A tax obligation record — one row per owner × property × period × type.
///
/// Persisted in the `tax_obligations` table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaxObligation {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub owner_id: Uuid,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub obligation_type: TaxObligationType,
    pub tax_period: TaxPeriod,
    /// Gross amount on which tax is computed (rent for MRI; fee for VAT/WHT).
    pub gross_amount_kes: Decimal,
    pub tax_kes: Decimal,
    pub tax_rate: Decimal,
    pub due_date: Date,
    pub status: TaxObligationStatus,
    /// KRA Payment Registration Number — populated after return is filed.
    pub kra_prn: Option<String>,
    /// KRA acknowledgement number from eRITS / iTax.
    pub kra_ack_number: Option<String>,
    pub filed_at: Option<OffsetDateTime>,
    pub remitted_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Command to create a new tax obligation.
#[derive(Clone, Debug)]
pub struct CreateTaxObligationCommand {
    pub agency_id: Uuid,
    pub owner_id: Uuid,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub obligation_type: TaxObligationType,
    pub tax_period: TaxPeriod,
    pub gross_amount_kes: Decimal,
    pub tax_kes: Decimal,
    pub tax_rate: Decimal,
    pub due_date: Date,
}

/// Command to mark a tax obligation as filed with KRA.
#[derive(Clone, Debug)]
pub struct MarkTaxFiledCommand {
    pub obligation_id: Uuid,
    pub kra_ack_number: String,
    pub filed_at: OffsetDateTime,
}

/// Command to mark a tax obligation as paid (PRN used).
#[derive(Clone, Debug)]
pub struct MarkTaxPaidCommand {
    pub obligation_id: Uuid,
    pub kra_prn: String,
    pub remitted_at: OffsetDateTime,
}

// ── KRA PIN validation result ─────────────────────────────────────────────────

/// Result from the GavaConnect PIN Checker API.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct KraPinVerificationResult {
    pub kra_pin: String,
    pub taxpayer_name: Option<String>,
    pub pin_valid: bool,
    /// True if the taxpayer has an active Tax Compliance Certificate.
    pub tcc_valid: Option<bool>,
    /// Tax obligations registered against this PIN (e.g. VAT, MRI, PAYE).
    pub obligations: Vec<String>,
    pub verified_at: OffsetDateTime,
}

// ── Annual income tracker ─────────────────────────────────────────────────────

/// Tracks a property owner's rolling 12-month gross rental income
/// to determine which MRI regime applies for each new period.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OwnerAnnualRentalIncome {
    pub owner_id: Uuid,
    pub year: i32,
    pub total_gross_rent_kes: Decimal,
    pub mri_regime: MriRegime,
    pub elected_normal_regime: bool,
}

impl OwnerAnnualRentalIncome {
    pub fn determine_regime(&self) -> MriRegime {
        if self.elected_normal_regime {
            return MriRegime::ElectedNormal;
        }
        if self.total_gross_rent_kes < MRI_ANNUAL_LOWER_THRESHOLD_KES {
            MriRegime::Exempt
        } else if self.total_gross_rent_kes > MRI_ANNUAL_UPPER_THRESHOLD_KES {
            MriRegime::NormalIncomeTax
        } else {
            MriRegime::Mri
        }
    }
}

// ── eTIMS invoice reference ───────────────────────────────────────────────────

/// Metadata returned by KRA's eTIMS after an invoice is submitted.
/// Stored alongside the Invoice record once the API is live.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct EtimsInvoiceRef {
    /// KRA-assigned invoice control unit number.
    pub cu_invoice_number: String,
    /// Signed QR-code payload to be printed on the invoice.
    pub qr_code: String,
    /// Timestamp KRA received and accepted the invoice.
    pub accepted_at: OffsetDateTime,
    /// eTIMS device / serial number used for the submission.
    pub device_serial: String,
}

// ── eRITS return payload ──────────────────────────────────────────────────────

/// Payload sent to KRA's eRITS endpoint (via GavaConnect) for one
/// property × period MRI return.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EritsReturnPayload {
    pub owner_kra_pin: String,
    pub property_id_kra: String, // KRA property reference once registered on eRITS
    pub tenant_kra_pin: Option<String>,
    pub tax_period: TaxPeriod,
    /// Gross rent received (no deductions allowed under MRI regime).
    pub gross_rent_kes: Decimal,
    /// Tax computed: gross_rent_kes × 7.5 %.
    pub tax_kes: Decimal,
    /// True when no rent was received this period.
    pub is_nil_return: bool,
}

/// Acknowledgement returned by eRITS after a successful return submission.
#[derive(Clone, Debug, Deserialize)]
pub struct EritsReturnAck {
    pub ack_number: String,
    pub prn: String, // Payment Registration Number
    pub accepted_at: OffsetDateTime,
    pub due_date: Date,
}

// ── Compliance summary ────────────────────────────────────────────────────────

/// Rolled-up compliance view for a single agency for a given period.
/// Used to power the compliance dashboard.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TaxComplianceSummary {
    pub agency_id: Uuid,
    pub tax_period: TaxPeriod,
    pub total_mri_kes: Decimal,
    pub total_vat_kes: Decimal,
    pub total_wht_kes: Decimal,
    pub obligations_total: u64,
    pub obligations_filed: u64,
    pub obligations_paid: u64,
    pub obligations_overdue: u64,
    pub mri_due_date: Date,
    pub vat_due_date: Date,
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn period() -> TaxPeriod {
        TaxPeriod::new(2025, 4)
    }

    #[test]
    fn mri_rate_is_7_5_percent() {
        let bd = TaxBreakdown::compute(
            Decimal::from(100_000),   // gross rent KES 100k/month
            Decimal::from(1_200_000), // annual = 1.2M (in MRI band)
            Decimal::from(10_000),    // management fee 10k
            true,                     // VAT-registered agency
            true,                     // WHT agent
            period(),
        );
        assert_eq!(bd.mri_kes, Decimal::new(750000, 2)); // KES 7,500
        assert_eq!(bd.mri_regime, Some(MriRegime::Mri));
    }

    #[test]
    fn mri_exempt_below_threshold() {
        let bd = TaxBreakdown::compute(
            Decimal::from(20_000),  // only KES 20k/month
            Decimal::from(240_000), // annual 240k < 288k → exempt
            Decimal::from(2_000),
            false,
            true,
            period(),
        );
        assert_eq!(bd.mri_kes, Decimal::ZERO);
        assert_eq!(bd.mri_regime, Some(MriRegime::Exempt));
    }

    #[test]
    fn mri_not_deducted_when_not_wht_agent() {
        let bd = TaxBreakdown::compute(
            Decimal::from(100_000),
            Decimal::from(1_200_000),
            Decimal::from(10_000),
            true,
            false, // not a WHT agent → owner self-files
            period(),
        );
        assert_eq!(bd.mri_kes, Decimal::ZERO);
    }

    #[test]
    fn vat_on_management_fee() {
        let bd = TaxBreakdown::compute(
            Decimal::from(100_000),
            Decimal::from(1_200_000),
            Decimal::from(10_000),
            true, // VAT-registered
            false,
            period(),
        );
        // 16% × 10,000 = 1,600
        assert_eq!(bd.vat_kes, Decimal::new(160000, 2));
    }

    #[test]
    fn wht_always_deducted_on_management_fee() {
        let bd = TaxBreakdown::compute(
            Decimal::ZERO,
            Decimal::from(1_200_000),
            Decimal::from(10_000),
            false,
            false,
            period(),
        );
        // 5% × 10,000 = 500
        assert_eq!(bd.wht_kes, Decimal::new(50000, 2));
    }

    #[test]
    fn net_payable_is_fee_plus_vat_minus_wht() {
        let bd = TaxBreakdown::compute(
            Decimal::from(100_000),
            Decimal::from(1_200_000),
            Decimal::from(10_000), // fee
            true,                  // VAT → +1,600
            false,                 // WHT → -500
            period(),
        );
        // 10,000 + 1,600 - 500 = 11,100
        assert_eq!(bd.net_payable_kes, Decimal::new(1_110_000, 2));
    }

    #[test]
    fn mri_due_date_is_20th_of_following_month() {
        let p = TaxPeriod::new(2025, 4);
        assert_eq!(
            p.mri_due_date(),
            Date::from_calendar_date(2025, Month::May, 20).unwrap()
        );
    }

    #[test]
    fn mri_due_date_wraps_december() {
        let p = TaxPeriod::new(2025, 12);
        assert_eq!(
            p.mri_due_date(),
            Date::from_calendar_date(2026, Month::January, 20).unwrap()
        );
    }

    #[test]
    fn high_earner_uses_normal_income_tax_regime() {
        let bd = TaxBreakdown::compute(
            Decimal::from(1_500_000),  // 1.5M/month
            Decimal::from(18_000_000), // annual > 15M
            Decimal::from(150_000),
            true,
            true,
            period(),
        );
        // MRI doesn't apply; owner must file normal income tax separately
        assert_eq!(bd.mri_kes, Decimal::ZERO);
        assert_eq!(bd.mri_regime, Some(MriRegime::NormalIncomeTax));
    }

    #[test]
    fn late_penalty_is_max_of_5pct_or_2000() {
        let bd = TaxBreakdown::compute(
            Decimal::from(10_000), // tiny rent → 7.5% = 750 MRI
            Decimal::from(500_000),
            Decimal::ZERO,
            false,
            true,
            period(),
        );
        // 5% of 750 = 37.50 → penalty should be KES 2,000 (minimum)
        assert_eq!(bd.late_filing_penalty_individual(), Decimal::from(2_000));
    }
}
