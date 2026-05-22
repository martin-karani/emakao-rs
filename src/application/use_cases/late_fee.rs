// src/application/use_cases/late_fee.rs
//
// Charges a late fee for an agreement, called by the WorkflowEngine when a
// "charge_late_fee" action fires.
//
// The action params JSON shape:
//   { "agreement_id": "<uuid>", "policy_id": "<uuid optional>" }
//
// If `policy_id` is absent the default policy for the agency is used.

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::finance::strategies::{late_fee_strategy, LateFeeContext, LateFeePolicy},
    infrastructure::audit::AuditEvent,
    presentation::app_state::AppState,
};

pub async fn charge_for_agreement(
    state: &Arc<AppState>,
    params: &serde_json::Value,
) -> anyhow::Result<()> {
    // ── 1. Parse params ───────────────────────────────────────────────────────
    let agreement_id: Uuid = params["agreement_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("charge_late_fee: missing agreement_id"))?
        .parse()?;

    let policy_id: Option<Uuid> = params["policy_id"].as_str().and_then(|s| s.parse().ok());

    // ── 2. Load agreement summary ─────────────────────────────────────────────
    // (Replace with your actual agreement repo call)
    let agreement = sqlx::query!(
        r#"
        SELECT agency_id, rent_amount_kes, outstanding_kes, days_overdue
        FROM   agreements_summary
        WHERE  id = $1
        "#,
        agreement_id,
    )
    .fetch_one(&state.db)
    .await?;

    // ── 3. Load late-fee policy ───────────────────────────────────────────────
    let policy_row = if let Some(pid) = policy_id {
        sqlx::query!(
            r#"SELECT fee_type, flat_amount_kes, percent_of_rent, max_fee_kes
               FROM late_fee_policy WHERE id = $1"#,
            pid,
        )
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query!(
            r#"SELECT fee_type, flat_amount_kes, percent_of_rent, max_fee_kes
               FROM late_fee_policy WHERE agency_id = $1 AND is_default = true"#,
            agreement.agency_id,
        )
        .fetch_one(&state.db)
        .await?
    };

    let policy = LateFeePolicy {
        fee_type: policy_row.fee_type,
        flat_amount_kes: policy_row.flat_amount_kes,
        percent_of_rent: policy_row.percent_of_rent,
        max_fee_kes: policy_row.max_fee_kes,
    };

    // ── 4. Calculate fee ──────────────────────────────────────────────────────
    let ctx = LateFeeContext {
        outstanding_kes: agreement.outstanding_kes.unwrap_or_default(),
        rent_amount_kes: agreement.rent_amount_kes,
        days_overdue: agreement.days_overdue.unwrap_or(0),
    };
    let fee = late_fee_strategy(&policy).calculate(&ctx);

    if fee.is_zero() {
        return Ok(());
    }

    // ── 5. Insert ledger entry ────────────────────────────────────────────────
    let entry_id = sqlx::query_scalar!(
        r#"
        INSERT INTO ledger_entries
            (agency_id, agreement_id, entry_type, amount_kes, posted_at)
        VALUES
            ($1, $2, 'late_fee', $3, now())
        RETURNING id
        "#,
        agreement.agency_id,
        agreement_id,
        fee,
    )
    .fetch_one(&state.db)
    .await?;

    // ── 6. Audit ──────────────────────────────────────────────────────────────
    state.audit.log(AuditEvent {
        agency_id: agreement.agency_id,
        actor_id: None,
        actor_role: Some("system".to_string()),
        action: "late_fee.charged".to_string(),
        entity_type: "ledger_entry".to_string(),
        entity_id: entry_id,
        old_data: None,
        new_data: Some(serde_json::json!({
            "agreement_id": agreement_id,
            "fee_kes":      fee,
        })),
        ip_address: None,
    });

    tracing::info!(
        agency    = %agreement.agency_id,
        agreement = %agreement_id,
        fee_kes   = %fee,
        "late fee charged"
    );

    Ok(())
}
