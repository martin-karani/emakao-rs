// src/infrastructure/jobs/workflow.rs
//
// WorkflowEngine — evaluates JSONLogic rules against a domain event and
// enqueues a delayed Apalis job for each matching rule.
//
// WorkflowJob (Apalis worker) — loads the rule's action array and dispatches
// each action: send_sms, send_email, create_task, charge_late_fee, flag_agreement.

use std::sync::Arc;

use apalis::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ── Job definition ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowJob {
    pub rule_id: Uuid,
    pub agency_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    /// The domain-event context used for JSONLogic evaluation and template rendering.
    pub context: serde_json::Value,
}

impl Job for WorkflowJob {
    const NAME: &'static str = "emakao::workflow_job";
}

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct WorkflowEngine {
    pool: PgPool,
    job_queue: apalis_redis::RedisStorage<WorkflowJob>,
}

impl WorkflowEngine {
    pub fn new(pool: PgPool, job_queue: apalis_redis::RedisStorage<WorkflowJob>) -> Self {
        Self { pool, job_queue }
    }

    /// Called by domain services when a domain event fires.
    /// Evaluates all active rules for the agency + event_type and enqueues
    /// matching ones with the configured delay.
    pub async fn trigger(
        &self,
        agency_id: Uuid,
        event_type: &str,
        entity_type: &str,
        entity_id: Uuid,
        context: serde_json::Value,
    ) -> anyhow::Result<()> {
        let rules = sqlx::query!(
            r#"
            SELECT id, conditions, offset_hours
            FROM   workflow_rules
            WHERE  agency_id  = $1
              AND  event_type = $2
              AND  is_active  = true
            "#,
            agency_id,
            event_type,
        )
        .fetch_all(&self.pool)
        .await?;

        for rule in rules {
            // Evaluate JSONLogic condition (empty object = always true).
            let conditions_empty = rule
                .conditions
                .as_object()
                .map(|o| o.is_empty())
                .unwrap_or(true);

            let matches = if conditions_empty {
                true
            } else {
                jsonlogic::apply(&rule.conditions, &context)
                    .map(|v| v.as_bool().unwrap_or(false))
                    .unwrap_or(false)
            };

            if !matches {
                continue;
            }

            let delay_secs = (rule.offset_hours.unwrap_or(0).max(0) as u64) * 3600;
            let job = WorkflowJob {
                rule_id: rule.id,
                agency_id,
                entity_type: entity_type.to_string(),
                entity_id,
                context: context.clone(),
            };

            if delay_secs == 0 {
                self.job_queue.clone().push(job).await?;
            } else {
                self.job_queue
                    .clone()
                    .push_delayed(job, std::time::Duration::from_secs(delay_secs))
                    .await?;
            }
        }

        Ok(())
    }
}

// ── Apalis worker ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct WorkflowAction {
    #[serde(rename = "type")]
    action_type: String,
    params: serde_json::Value,
}

pub async fn execute_workflow_job(
    job: WorkflowJob,
    Data(state): Data<Arc<crate::presentation::app_state::AppState>>,
) -> anyhow::Result<()> {
    let rule = sqlx::query!(
        "SELECT actions FROM workflow_rules WHERE id = $1",
        job.rule_id
    )
    .fetch_one(&state.db)
    .await?;

    let actions: Vec<WorkflowAction> = serde_json::from_value(rule.actions).unwrap_or_default();

    for action in actions {
        match action.action_type.as_str() {
            "send_sms" | "send_email" => {
                // Resolve recipient from context; channel from action_type.
                let channel = if action.action_type == "send_sms" {
                    "sms"
                } else {
                    "email"
                };
                dispatch_notification(&state, &job, channel, &action.params).await?;
            }
            "create_task" => {
                task_repo::create_from_params(&state.db, &action.params, &job.context).await?;
            }
            "charge_late_fee" => {
                crate::application::use_cases::late_fee::charge_for_agreement(
                    &state,
                    &action.params,
                )
                .await?;
            }
            "flag_agreement" => {
                agreement_repo::set_flag(&state.db, job.entity_id, &action.params).await?;
            }
            other => {
                tracing::warn!(
                    rule_id = %job.rule_id,
                    "unknown workflow action type: {other} — skipped"
                );
            }
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn dispatch_notification(
    state: &Arc<crate::presentation::app_state::AppState>,
    job: &WorkflowJob,
    channel: &str,
    params: &serde_json::Value,
) -> anyhow::Result<()> {
    // Extract optional template override from params.
    let event_key = params
        .get("template")
        .and_then(|v| v.as_str())
        .unwrap_or("workflow.generic");

    let phone = job
        .context
        .get("phone")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let email = job
        .context
        .get("email")
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    // Load CommunicationSettings from cache.
    let settings = state
        .settings_cache
        .get_or_load(job.agency_id, &state.db)
        .await?;

    state
        .notifications
        .dispatch(
            job.agency_id,
            event_key,
            crate::infrastructure::notifications::dispatcher::Recipient {
                phone,
                email,
                whatsapp: None,
            },
            job.context.clone(),
            &settings.communication,
        )
        .await
}

// ── Stub repos (replace with real implementations) ───────────────────────────

mod task_repo {
    use sqlx::PgPool;
    pub async fn create_from_params(
        _pool: &PgPool,
        _params: &serde_json::Value,
        _context: &serde_json::Value,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

mod agreement_repo {
    use sqlx::PgPool;
    use uuid::Uuid;
    pub async fn set_flag(
        _pool: &PgPool,
        _id: Uuid,
        _params: &serde_json::Value,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
