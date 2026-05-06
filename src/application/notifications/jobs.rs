//! Apalis job payloads for async email and SMS delivery.
//!
//! In apalis 1.0.0-rc.7 the `Job` trait is no longer required — job types
//! only need `Serialize + DeserializeOwned + Send + 'static`.
//!
//! Key asymmetry:
//! * `EmailJob` — carries the raw context + template variant; rendering
//!   happens *inside the worker* via minijinja so template fixes take effect
//!   on the next retry without re-queuing.
//! * `SmsJob` — carries the *already-rendered* message string. Rendering
//!   happened at enqueue time in `NotificationService::sms()`.

use serde::{Deserialize, Serialize};

use super::templates::EmailTemplate;

// ── Email Job ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailJob {
    pub to: String,
    pub template: EmailTemplate,
    pub context: serde_json::Value,
}

// ── SMS Job ───────────────────────────────────────────────────────────────────

/// Pre-rendered SMS — the worker just calls `sms_port.send(to, message)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsJob {
    pub to: String,
    pub message: String,
}
