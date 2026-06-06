// src/infrastructure/notifications/dispatcher.rs
//
// NotificationDispatcher — the single entry-point for sending any
// outbound notification (SMS, email, WhatsApp).
//
// Per-request flow:
//   1. Check quiet hours against CommunicationSettings.
//   2. Load per-event channel toggles from `agency_notification_toggles`.
//   3. For each enabled channel, render the MiniJinja template
//      (agency override → system default fallback).
//   4. Dispatch via the ProviderRegistry.

use std::sync::Arc;

use minijinja::Environment;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::agency_settings::CommunicationSettings,
    infrastructure::providers::{EmailMessage, ProviderRegistry},
};

// ── Types ─────────────────────────────────────────────────────────────────────

pub struct Recipient {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
}

#[derive(Debug)]
struct ToggleRow {
    sms_enabled: bool,
    email_enabled: bool,
    whatsapp_enabled: bool,
}

// ── Dispatcher ────────────────────────────────────────────────────────────────

pub struct NotificationDispatcher {
    pub pool: PgPool,
    pub providers: Arc<ProviderRegistry>,
    pub jinja: Environment<'static>,
}

impl NotificationDispatcher {
    // ── Public API ────────────────────────────────────────────────────────────

    pub async fn dispatch(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        event_key: &str,
        recipient: Recipient,
        ctx: serde_json::Value,
        settings: &CommunicationSettings,
    ) -> anyhow::Result<()> {
        // 1. Quiet hours
        let hour = time::OffsetDateTime::now_utc().hour();
        if self.in_quiet_hours(hour, settings) {
            tracing::debug!(
                agency = %agency_id,
                event  = event_key,
                "notification suppressed: quiet hours"
            );
            return Ok(());
        }

        // 2. Per-event toggles
        let toggle = self.get_toggle(agency_id, event_key).await?;

        // 3. SMS
        if toggle.sms_enabled {
            if let Some(phone) = &recipient.phone {
                let body = self
                    .render(agency_id, property_id, "sms", event_key, &ctx)
                    .await?;
                let provider = self.providers.sms.for_agency(agency_id);
                let sender_id = settings
                    .sms_trigger_rules
                    .iter()
                    .find(|r| r.event_key == event_key)
                    .map(|_| ""); // real sender_id comes from BrandingSettings
                provider.send(phone, &body, sender_id).await?;
            }
        }

        // 4. Email
        if toggle.email_enabled {
            if let Some(to) = &recipient.email {
                let body = self
                    .render(agency_id, property_id, "email", event_key, &ctx)
                    .await?;
                let subject = self
                    .render_subject(agency_id, property_id, event_key, &ctx)
                    .await?;
                if let Some(provider) = self.providers.email.get(&agency_id) {
                    provider
                        .send(EmailMessage {
                            to: to.clone(),
                            subject,
                            body,
                            text: None,
                            reply_to: None,
                        })
                        .await?;
                }
            }
        }

        // 5. WhatsApp (forward-looking; same MiniJinja path as SMS)
        if toggle.whatsapp_enabled {
            if let Some(_wa) = &recipient.whatsapp {
                // TODO: wire up WhatsApp provider when available
                tracing::debug!("whatsapp dispatch not yet implemented");
            }
        }

        Ok(())
    }

    // ── Internals ─────────────────────────────────────────────────────────────

    async fn get_toggle(&self, agency_id: Uuid, event_key: &str) -> anyhow::Result<ToggleRow> {
        // Fall back to sensible defaults when no row exists (SMS+email on, WA off).
        let row = sqlx::query(
            r#"
            SELECT sms_enabled, email_enabled, whatsapp_enabled
            FROM   agency_notification_toggles
            WHERE  agency_id = $1 AND event_key = $2
            "#,
        )
        .bind(agency_id)
        .bind(event_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(r) => {
                use sqlx::Row;
                ToggleRow {
                    sms_enabled: r.try_get("sms_enabled").unwrap_or(true),
                    email_enabled: r.try_get("email_enabled").unwrap_or(true),
                    whatsapp_enabled: r.try_get("whatsapp_enabled").unwrap_or(false),
                }
            }
            None => ToggleRow {
                sms_enabled: true,
                email_enabled: true,
                whatsapp_enabled: false,
            },
        })
    }

    /// Property template override → Agency template override → system default.
    async fn render(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        channel: &str,
        event_key: &str,
        ctx: &serde_json::Value,
    ) -> anyhow::Result<String> {
        let tmpl_str = sqlx::query_scalar(
            r#"
            SELECT body
            FROM   notification_templates
            WHERE  agency_id = $1
              AND  channel   = $2
              AND  event_key = $3
              AND  (property_id IS NULL OR property_id = $4)
            ORDER BY property_id DESC NULLS LAST
            LIMIT 1
            "#,
        )
        .bind(agency_id)
        .bind(channel)
        .bind(event_key)
        .bind(property_id)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_else(|| self.system_default(channel, event_key));

        let rendered = self.jinja.template_from_str(&tmpl_str)?.render(ctx)?;
        Ok(rendered)
    }

    async fn render_subject(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        event_key: &str,
        ctx: &serde_json::Value,
    ) -> anyhow::Result<String> {
        let subj_tmpl = sqlx::query_scalar(
            r#"
            SELECT subject
            FROM   notification_templates
            WHERE  agency_id = $1
              AND  channel   = 'email'
              AND  event_key = $2
              AND  (property_id IS NULL OR property_id = $3)
            ORDER BY property_id DESC NULLS LAST
            LIMIT 1
            "#,
        )
        .bind(agency_id)
        .bind(event_key)
        .bind(property_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten()
        .unwrap_or_else(|| format!("Notification: {event_key}"));

        Ok(self.jinja.template_from_str(&subj_tmpl)?.render(ctx)?)
    }

    /// Embedded system-default templates.  Override per-channel below.
    fn system_default(&self, channel: &str, event_key: &str) -> String {
        match (channel, event_key) {
            ("sms", "rent.overdue") => "Dear {{ tenant_name }}, your rent of KES {{ amount }} \
                 was due on {{ due_date }}. Please pay promptly."
                .to_string(),
            ("email", "rent.overdue") => "<p>Dear {{ tenant_name }},</p>\
                 <p>Your rent of <strong>KES {{ amount }}</strong> \
                 was due on {{ due_date }}.</p>"
                .to_string(),
            ("sms", "lease.expiry_notice") => {
                "Dear {{ tenant_name }}, your lease expires on {{ expiry_date }}. \
                 Please contact us to discuss renewal."
                    .to_string()
            }
            ("sms", "invoice.new") => {
                "Dear {{ tenant_name }}, a new invoice #{{ invoice_number }} of KES {{ amount }} \
                 has been generated. Due date: {{ due_date }}. Thank you."
                    .to_string()
            }
            ("email", "invoice.new") => {
                "<h3>Invoice #{{ invoice_number }}</h3>\
                 <p>Dear {{ tenant_name }},</p>\
                 <p>An invoice for <strong>KES {{ amount }}</strong> has been generated.</p>\
                 <p>Due date: {{ due_date }}</p>\
                 <p>Breakdown:</p><ul>\
                 {% for item in line_items %}<li>{{ item.description }}: KES {{ item.total_kes }}</li>{% endfor %}\
                 </ul>"
                     .to_string()
             }
            ("sms", "manual.notice") => "{{ notice_body }}".to_string(),
            ("email", "manual.notice") => "<h3>{{ notice_subject }}</h3><p>{{ notice_body }}</p>".to_string(),
            ("sms", "billing.statement") => {
                "Your Acct {{ acct_no }} {{ month }} Unit {{ unit }}, {{ prev_month }} Arrears: {{ arrears }} \
                 plus {{ month }} Rent= {{ rent }} ; Water Bill= {{ water }} ; Garbage= {{ garbage }} \
                 Total Bal Ksh {{ total }} as at {{ today }}. Payable by 5th-{{ month }}-{{ year }}"
                    .to_string()
            }
            _ => "{{ event_key }} notification for {{ agency_name }}".to_string(),
        }
    }

    fn in_quiet_hours(&self, hour: u8, s: &CommunicationSettings) -> bool {
        let start = s.quiet_start_hour;
        let end = s.quiet_end_hour;
        if end > start {
            hour < start || hour >= end
        } else {
            false
        }
    }
}
