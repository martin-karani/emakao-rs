// src/application/use_cases/tax/compute_obligations.rs
//
// Generates MRI / VAT / WHT tax obligation rows for a closed calendar month.
//
// Called by the tax compliance worker on the 1st of each month for the
// just-closed period.  Uses `AgreementRepository::find_active_for_agency`
// which returns `TaxAgreementView` — a lightweight projection that includes
// `owner_id` and `owner_kra_pin` resolved via the agreements→units→properties
// join, avoiding a separate property/owner lookup per agreement.

use std::sync::Arc;

use rust_decimal::Decimal;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{agreement_repository::AgreementRepository, tax_repository::TaxRepository},
    },
    domain::tax::{
        CreateTaxObligationCommand, MriRegime, OwnerAnnualRentalIncome, TaxPeriod,
        MRI_ANNUAL_LOWER_THRESHOLD_KES, MRI_ANNUAL_UPPER_THRESHOLD_KES, MRI_RATE, VAT_RATE,
        WHT_MANAGEMENT_FEE_RATE,
    },
};

// ── Input / Output ────────────────────────────────────────────────────────────

pub struct ComputeObligationsInput {
    pub agency_id: Uuid,
    /// The closed calendar month to compute obligations for.
    pub tax_period: TaxPeriod,
    /// True when the agency is a KRA-appointed WHT agent for rental income.
    pub agency_is_wht_agent: bool,
    /// True when the agency is VAT-registered (turnover > KES 5 M/yr).
    pub agency_is_vat_registered: bool,
    /// Management fee rate as a fraction, e.g. `0.10` = 10 %.
    pub management_fee_rate: Decimal,
}

#[derive(Debug)]
pub struct ComputeObligationsResult {
    pub obligations_created: u64,
    pub obligations_skipped: u64,
}

// ── Use case ──────────────────────────────────────────────────────────────────

pub struct ComputeObligationsUseCase {
    pub agreement_repo: Arc<dyn AgreementRepository>,
    pub tax_repo: Arc<dyn TaxRepository>,
}

impl ComputeObligationsUseCase {
    pub fn new(
        agreement_repo: Arc<dyn AgreementRepository>,
        tax_repo: Arc<dyn TaxRepository>,
    ) -> Self {
        Self {
            agreement_repo,
            tax_repo,
        }
    }

    pub async fn execute(
        &self,
        input: ComputeObligationsInput,
    ) -> Result<ComputeObligationsResult, AppError> {
        // First day of the tax period — used as the period key in the DB.
        let period_date = Date::from_calendar_date(
            input.tax_period.year,
            time::Month::try_from(input.tax_period.month)
                .map_err(|_| AppError::Validation("invalid tax period month".into()))?,
            1,
        )
        .map_err(|_| AppError::Validation("invalid tax period date".into()))?;

        let mri_due = input.tax_period.mri_due_date(); // 20th of following month
        let wht_due = input.tax_period.wht_agent_due_date(); // 5th of following month

        // `find_active_for_agency` joins through units → properties so that
        // `owner_id` and `owner_kra_pin` are already resolved.
        let agreements = self
            .agreement_repo
            .find_active_for_agency(input.agency_id)
            .await?;

        let mut created = 0u64;
        let mut skipped = 0u64;

        for ag in &agreements {
            // ── Determine MRI regime for this owner ───────────────────────
            let annual = self
                .tax_repo
                .get_annual_income(input.agency_id, ag.owner_id, input.tax_period.year)
                .await?;

            // If no annual record yet, project from the monthly rent.
            let annual_kes = annual
                .as_ref()
                .map(|a| a.total_gross_rent_kes)
                .unwrap_or_else(|| ag.rent_amount_kes * Decimal::from(12));

            let regime = determine_regime(annual_kes, annual.as_ref());

            // ── 1. MRI obligation ─────────────────────────────────────────
            // Only when the agency is a WHT agent AND the owner is in the MRI band.
            if regime == MriRegime::Mri && input.agency_is_wht_agent {
                let mri_kes = (ag.rent_amount_kes * MRI_RATE).round_dp(2);

                if self
                    .obligation_exists(
                        &input,
                        ag.owner_id,
                        ag.property_id,
                        period_date,
                        crate::domain::tax::TaxObligationType::Mri,
                    )
                    .await?
                {
                    skipped += 1;
                } else {
                    self.tax_repo
                        .create_obligation(CreateTaxObligationCommand {
                            agency_id: input.agency_id,
                            owner_id: ag.owner_id,
                            property_id: ag.property_id,
                            agreement_id: Some(ag.id),
                            obligation_type: crate::domain::tax::TaxObligationType::Mri,
                            tax_period: input.tax_period,
                            gross_amount_kes: ag.rent_amount_kes,
                            tax_kes: mri_kes,
                            tax_rate: MRI_RATE,
                            due_date: wht_due,
                        })
                        .await?;
                    created += 1;
                    tracing::info!(
                        agreement_id = %ag.id,
                        owner_id     = %ag.owner_id,
                        mri_kes      = %mri_kes,
                        period       = %input.tax_period,
                        "MRI obligation created"
                    );
                }
            }

            // ── 2. VAT obligation (on management fee) ─────────────────────
            if input.agency_is_vat_registered {
                let mgmt_fee = (ag.rent_amount_kes * input.management_fee_rate).round_dp(2);
                let vat_kes = (mgmt_fee * VAT_RATE).round_dp(2);

                if self
                    .obligation_exists(
                        &input,
                        ag.owner_id,
                        ag.property_id,
                        period_date,
                        crate::domain::tax::TaxObligationType::Vat,
                    )
                    .await?
                {
                    skipped += 1;
                } else {
                    self.tax_repo
                        .create_obligation(CreateTaxObligationCommand {
                            agency_id: input.agency_id,
                            owner_id: ag.owner_id,
                            property_id: ag.property_id,
                            agreement_id: Some(ag.id),
                            obligation_type: crate::domain::tax::TaxObligationType::Vat,
                            tax_period: input.tax_period,
                            gross_amount_kes: mgmt_fee,
                            tax_kes: vat_kes,
                            tax_rate: VAT_RATE,
                            due_date: mri_due,
                        })
                        .await?;
                    created += 1;
                }
            }

            // ── 3. WHT obligation (5 % deducted by owner on management fee)
            {
                let mgmt_fee = (ag.rent_amount_kes * input.management_fee_rate).round_dp(2);
                let wht_kes = (mgmt_fee * WHT_MANAGEMENT_FEE_RATE).round_dp(2);

                if self
                    .obligation_exists(
                        &input,
                        ag.owner_id,
                        ag.property_id,
                        period_date,
                        crate::domain::tax::TaxObligationType::Wht,
                    )
                    .await?
                {
                    skipped += 1;
                } else {
                    self.tax_repo
                        .create_obligation(CreateTaxObligationCommand {
                            agency_id: input.agency_id,
                            owner_id: ag.owner_id,
                            property_id: ag.property_id,
                            agreement_id: Some(ag.id),
                            obligation_type: crate::domain::tax::TaxObligationType::Wht,
                            tax_period: input.tax_period,
                            gross_amount_kes: mgmt_fee,
                            tax_kes: wht_kes,
                            tax_rate: WHT_MANAGEMENT_FEE_RATE,
                            due_date: wht_due,
                        })
                        .await?;
                    created += 1;
                }
            }

            // ── 4. Update rolling annual income ───────────────────────────
            let new_annual = annual
                .as_ref()
                .map(|a| a.total_gross_rent_kes)
                .unwrap_or(Decimal::ZERO)
                + ag.rent_amount_kes;

            let new_regime = determine_regime(new_annual, annual.as_ref());

            self.tax_repo
                .upsert_annual_income(
                    input.agency_id,
                    ag.owner_id,
                    input.tax_period.year,
                    new_annual,
                    new_regime,
                )
                .await?;
        }

        Ok(ComputeObligationsResult {
            obligations_created: created,
            obligations_skipped: skipped,
        })
    }

    // ── Helper — thin wrapper to avoid repeating the full call ────────────────

    async fn obligation_exists(
        &self,
        input: &ComputeObligationsInput,
        owner_id: Uuid,
        property_id: Uuid,
        period_date: Date,
        obligation_type: crate::domain::tax::TaxObligationType,
    ) -> Result<bool, AppError> {
        self.tax_repo
            .obligation_exists(
                input.agency_id,
                owner_id,
                property_id,
                period_date,
                obligation_type,
            )
            .await
    }
}

// ── Regime helper ─────────────────────────────────────────────────────────────

fn determine_regime(annual_kes: Decimal, existing: Option<&OwnerAnnualRentalIncome>) -> MriRegime {
    if existing.map(|e| e.elected_normal_regime).unwrap_or(false) {
        return MriRegime::ElectedNormal;
    }
    if annual_kes < MRI_ANNUAL_LOWER_THRESHOLD_KES {
        MriRegime::Exempt
    } else if annual_kes > MRI_ANNUAL_UPPER_THRESHOLD_KES {
        MriRegime::NormalIncomeTax
    } else {
        MriRegime::Mri
    }
}
