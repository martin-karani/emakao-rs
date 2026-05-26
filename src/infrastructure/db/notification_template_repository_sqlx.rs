// src/infrastructure/db/notification_template_repository_sqlx.rs

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::notification_template_repository::{
            ListTemplatesFilter, NotificationTemplateRepository, UpsertTemplateCommand,
        },
    },
    domain::notification_template::NotificationTemplate,
};

pub struct PgNotificationTemplateRepo {
    pool: PgPool,
}

impl PgNotificationTemplateRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Internal row ──────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct NotificationTemplateRow {
    id: Uuid,
    agency_id: Uuid,
    channel: String,
    event_key: String,
    locale: String,
    subject: Option<String>,
    body: String,
    created_at: time::OffsetDateTime,
}

impl From<NotificationTemplateRow> for NotificationTemplate {
    fn from(r: NotificationTemplateRow) -> Self {
        Self {
            id: r.id,
            agency_id: r.agency_id,
            channel: r.channel,
            event_key: r.event_key,
            locale: r.locale,
            subject: r.subject,
            body: r.body,
            created_at: r.created_at,
        }
    }
}

// ── Implementation ────────────────────────────────────────────────────────────

#[async_trait]
impl NotificationTemplateRepository for PgNotificationTemplateRepo {
    async fn list(
        &self,
        filter: ListTemplatesFilter,
    ) -> Result<Vec<NotificationTemplate>, AppError> {
        let rows = sqlx::query_as::<_, NotificationTemplateRow>(
            r#"
            SELECT id, agency_id, channel, event_key, locale, subject, body, created_at
            FROM   notification_templates
            WHERE  agency_id = $1
              AND  ($2::text IS NULL OR channel   = $2)
              AND  ($3::text IS NULL OR event_key = $3)
            ORDER  BY channel, event_key, locale
            "#,
        )
        .bind(filter.agency_id)
        .bind(filter.channel)
        .bind(filter.event_key)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(NotificationTemplate::from).collect())
    }

    async fn upsert(&self, cmd: UpsertTemplateCommand) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO notification_templates
                (agency_id, channel, event_key, locale, subject, body)
            VALUES
                ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (agency_id, channel, event_key, locale)
            DO UPDATE SET
                subject = EXCLUDED.subject,
                body    = EXCLUDED.body
            "#,
        )
        .bind(cmd.agency_id)
        .bind(cmd.channel)
        .bind(cmd.event_key)
        .bind(cmd.locale)
        .bind(cmd.subject)
        .bind(cmd.body)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn delete(
        &self,
        agency_id: Uuid,
        channel: &str,
        event_key: &str,
        locale: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM notification_templates
            WHERE agency_id = $1
              AND channel   = $2
              AND event_key = $3
              AND locale    = $4
            "#,
        )
        .bind(agency_id)
        .bind(channel)
        .bind(event_key)
        .bind(locale)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}
