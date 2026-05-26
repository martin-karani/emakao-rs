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

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    application::finance::strategies::{late_fee_strategy, LateFeeContext, LateFeePolicy},
    infrastructure::audit::AuditEvent,
    presentation::app_state::AppState,
};

pub async fn charge_for_agreement(
    state: &Arc<AppState>,
    pool: &PgPool,
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
    let agreement = sqlx::query(
        r#"
        SELECT agency_id, id, rent_amount_kes, outstanding_kes, days_overdue
        FROM   agreements_summary
        WHERE  id = $1
        "#,
    )
    .bind(agreement_id)
    .fetch_one(pool)
    .await?;

    // ── 3. Load late-fee policy ───────────────────────────────────────────────
    let policy_row = if let Some(pid) = policy_id {
        sqlx::query(
            r#"SELECT fee_type, flat_amount_kes, percent_of_rent, max_fee_kes
               FROM late_fee_policy WHERE id = $1"#,
        )
        .bind(pid)
        .fetch_one(pool)
        .await?
    } else {
        let agency_id_val: Uuid = agreement.try_get("agency_id")?;
        sqlx::query(
            r#"SELECT fee_type, flat_amount_kes, percent_of_rent, max_fee_kes
               FROM late_fee_policy WHERE agency_id = $1 AND is_default = true"#,
        )
        .bind(agency_id_val)
        .fetch_one(pool)
        .await?
    };

    let policy = LateFeePolicy {
        fee_type: policy_row.try_get("fee_type")?,
        flat_amount_kes: policy_row.try_get("flat_amount_kes")?,
        percent_of_rent: policy_row.try_get("percent_of_rent")?,
        max_fee_kes: policy_row.try_get("max_fee_kes")?,
    };

    let agency_id: Uuid = agreement.try_get("agency_id")?;
    let agreement_db_id: Uuid = agreement.try_get("id")?;

    // ── 4. Calculate fee ──────────────────────────────────────────────────────
    let ctx = LateFeeContext {
        outstanding_kes: agreement
            .try_get::<Option<rust_decimal::Decimal>, _>("outstanding_kes")?
            .unwrap_or_default(),
        rent_amount_kes: agreement.try_get("rent_amount_kes")?,
        days_overdue: agreement
            .try_get::<Option<i32>, _>("days_overdue")?
            .unwrap_or(0),
    };
    let fee = late_fee_strategy(&policy).calculate(&ctx);

    if fee.is_zero() {
        return Ok(());
    }

    // ── 5. Insert ledger entry ────────────────────────────────────────────────
    let entry_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO ledger_entries
            (agency_id, agreement_id, entry_type, amount_kes, posted_at)
        VALUES
            ($1, $2, 'fee_late', $3, now())
        RETURNING id
        "#,
    )
    .bind(agency_id)
    .bind(agreement_db_id)
    .bind(fee)
    .fetch_one(pool)
    .await?;

    // ── 6. Audit ──────────────────────────────────────────────────────────────
    state.customisation().audit.log(AuditEvent {
        agency_id,
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
        agency    = %agency_id,
        agreement = %agreement_id,
        fee_kes   = %fee,
        "late fee charged"
    );

    Ok(())
}
