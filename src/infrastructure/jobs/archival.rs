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

// ── Job definition ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalJob {
    pub agency_id: Uuid,
}

impl Job for ArchivalJob {
    const NAME: &'static str = "emakao::archival_job";
}

// ── DB projection ─────────────────────────────────────────────────────────────

struct DataRetentionPolicy {
    financial_retain_years: i16,
    cold_storage_bucket: Option<String>,
}

// ── Worker ────────────────────────────────────────────────────────────────────

pub async fn process_archival(
    job: ArchivalJob,
    Data(state): Data<Arc<crate::presentation::app_state::AppState>>,
) -> anyhow::Result<()> {
    let policy = load_policy(&state.db, job.agency_id).await?;

    let cutoff =
        chrono::Utc::now() - chrono::Duration::days(policy.financial_retain_years as i64 * 365);

    // Load IDs in chunks to avoid huge IN lists.
    let stale_ids: Vec<Uuid> = sqlx::query_scalar!(
        r#"
        SELECT id
        FROM   ledger_entries
        WHERE  agency_id  = $1
          AND  posted_at  < $2
          AND  deleted_at IS NULL
        "#,
        job.agency_id,
        cutoff,
    )
    .fetch_all(&state.db)
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
        let archive_key = if let Some(bucket) = &policy.cold_storage_bucket {
            let key = format!(
                "archival/{}/{}.jsonl",
                job.agency_id,
                chrono::Utc::now().format("%Y%m%d%H%M%S"),
            );
            export_chunk_to_s3(&state.s3, bucket, &key, &state.db, chunk).await?;
            Some(key)
        } else {
            None
        };

        // 2. Soft-delete in Postgres.
        sqlx::query!(
            r#"
            UPDATE ledger_entries
            SET    deleted_at = now()
            WHERE  id         = ANY($1)
            "#,
            chunk as &[Uuid],
        )
        .execute(&state.db)
        .await?;

        // 3. Write archival_log entries.
        for &entity_id in chunk {
            sqlx::query!(
                r#"
                INSERT INTO archival_log
                    (agency_id, entity_type, entity_id, action, archive_key, performed_by)
                VALUES
                    ($1, 'ledger_entry', $2, 'archived', $3, NULL)
                "#,
                job.agency_id,
                entity_id,
                archive_key.as_deref(),
            )
            .execute(&state.db)
            .await?;
        }
    }

    tracing::info!(agency = %job.agency_id, "archival: complete");
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn load_policy(pool: &PgPool, agency_id: Uuid) -> anyhow::Result<DataRetentionPolicy> {
    let row = sqlx::query!(
        r#"
        SELECT financial_retain_years, cold_storage_bucket
        FROM   data_retention_policy
        WHERE  agency_id = $1
        "#,
        agency_id,
    )
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some(r) => DataRetentionPolicy {
            financial_retain_years: r.financial_retain_years,
            cold_storage_bucket: r.cold_storage_bucket,
        },
        // Fallback to KRA default when no policy row exists.
        None => DataRetentionPolicy {
            financial_retain_years: 7,
            cold_storage_bucket: None,
        },
    })
}

/// Fetch rows for `ids` and write them as JSONL to an S3 key.
async fn export_chunk_to_s3(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    pool: &PgPool,
    ids: &[Uuid],
) -> anyhow::Result<()> {
    let rows = sqlx::query!(
        r#"
        SELECT id, agency_id, entry_type, amount_kes, posted_at, metadata
        FROM   ledger_entries
        WHERE  id = ANY($1)
        "#,
        ids as &[Uuid],
    )
    .fetch_all(pool)
    .await?;

    let jsonl: String = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":          r.id,
                "agency_id":   r.agency_id,
                "entry_type":  r.entry_type,
                "amount_kes":  r.amount_kes,
                "posted_at":   r.posted_at,
                "metadata":    r.metadata,
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    s3.put_object()
        .bucket(bucket)
        .key(key)
        .body(aws_sdk_s3::primitives::ByteStream::from(jsonl.into_bytes()))
        .content_type("application/x-ndjson")
        .send()
        .await?;

    Ok(())
}
