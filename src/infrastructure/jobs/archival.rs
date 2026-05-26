// src/infrastructure/jobs/archival.rs
//
// ArchivalJob — runs on a schedule (e.g. monthly) and:
//   1. Loads the agency's DataRetentionPolicy.
//   2. Finds ledger entries older than the retention window.
//   3. Exports them to S3 Glacier (if cold_storage_bucket is set).
//   4. Soft-deletes the rows in Postgres.
//   5. Writes a record to `archival_log`.

use std::sync::Arc;

use apalis::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::ports::storage_port::StoragePort;

// ── Job definition ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalJob {
    pub agency_id: Uuid,
}

// ── DB projection ─────────────────────────────────────────────────────────────

struct DataRetentionPolicy {
    financial_retain_years: i16,
    cold_storage_bucket: Option<String>,
}

// ── Worker ────────────────────────────────────────────────────────────────────

pub async fn process_archival(
    job: ArchivalJob,
    state: Data<Arc<crate::presentation::app_state::AppState>>,
) -> anyhow::Result<()> {
    let pool = state.infra.tenant_pools.for_agency(job.agency_id).await?;
    let policy = load_policy(&pool, job.agency_id).await?;

    let cutoff = time::OffsetDateTime::now_utc()
        - time::Duration::days(policy.financial_retain_years as i64 * 365);

    // Load IDs in chunks to avoid huge IN lists.
    let stale_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM   ledger_entries
        WHERE  agency_id  = $1
          AND  posted_at  < $2
          AND  deleted_at IS NULL
        "#,
    )
    .bind(job.agency_id)
    .bind(cutoff)
    .fetch_all(&pool)
    .await?;

    if stale_ids.is_empty() {
        tracing::info!(agency = %job.agency_id, "archival: nothing to archive");
        return Ok(());
    }

    tracing::info!(
        agency = %job.agency_id,
        count  = stale_ids.len(),
        "archival: starting"
    );

    for chunk in stale_ids.chunks(500) {
        // 1. Export to S3 (if configured).
        let archive_key = if let Some(_bucket) = &policy.cold_storage_bucket {
            let format =
                time::format_description::parse("[year][month][day][hour][minute][second]")?;
            let key = format!(
                "archival/{}/{}.jsonl",
                job.agency_id,
                time::OffsetDateTime::now_utc().format(&format)?,
            );
            export_chunk_to_s3(state.storage.as_ref(), &key, &pool, chunk).await?;
            Some(key)
        } else {
            None
        };

        // 2. Soft-delete in Postgres.
        sqlx::query(
            r#"
            UPDATE ledger_entries
            SET    deleted_at = now()
            WHERE  id         = ANY($1)
            "#,
        )
        .bind(chunk)
        .execute(&pool)
        .await?;

        // 3. Write archival_log entries.
        for &entity_id in chunk {
            sqlx::query(
                r#"
                INSERT INTO archival_log
                    (agency_id, entity_type, entity_id, action, archive_key, performed_by)
                VALUES
                    ($1, 'ledger_entry', $2, 'archived', $3, NULL)
                "#,
            )
            .bind(job.agency_id)
            .bind(entity_id)
            .bind(archive_key.as_deref())
            .execute(&pool)
            .await?;
        }
    }

    tracing::info!(agency = %job.agency_id, "archival: complete");
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn load_policy(pool: &PgPool, agency_id: Uuid) -> anyhow::Result<DataRetentionPolicy> {
    let row = sqlx::query(
        r#"
        SELECT financial_retain_years, cold_storage_bucket
        FROM   data_retention_policy
        WHERE  agency_id = $1
        "#,
    )
    .bind(agency_id)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some(r) => {
            use sqlx::Row;
            DataRetentionPolicy {
                financial_retain_years: r.try_get("financial_retain_years").unwrap_or(7),
                cold_storage_bucket: r.try_get("cold_storage_bucket").unwrap_or_default(),
            }
        }
        // Fallback to KRA default when no policy row exists.
        None => DataRetentionPolicy {
            financial_retain_years: 7,
            cold_storage_bucket: None,
        },
    })
}

/// Fetch rows for `ids` and write them as JSONL to an S3 key.
async fn export_chunk_to_s3(
    storage: &dyn StoragePort,
    key: &str,
    pool: &PgPool,
    ids: &[Uuid],
) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT id, agency_id, entry_type, amount_kes, posted_at, metadata
        FROM   ledger_entries
        WHERE  id = ANY($1)
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;

    let jsonl: String = rows
        .into_iter()
        .map(|r| {
            use sqlx::Row;
            let meta: Option<serde_json::Value> = r.try_get("metadata").unwrap_or_default();
            serde_json::json!({
                "id":          r.try_get::<Uuid, _>("id").unwrap_or_default(),
                "agency_id":   r.try_get::<Uuid, _>("agency_id").unwrap_or_default(),
                "entry_type":  r.try_get::<String, _>("entry_type").unwrap_or_default(),
                "amount_kes":  r.try_get::<rust_decimal::Decimal, _>("amount_kes").unwrap_or_default(),
                "posted_at":   r.try_get::<time::OffsetDateTime, _>("posted_at").unwrap_or_else(|_| time::OffsetDateTime::now_utc()),
                "metadata":    meta,
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    storage
        .upload(key, jsonl.into_bytes(), "application/x-ndjson")
        .await?;

    Ok(())
}
