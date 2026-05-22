pub mod registry;
pub mod traits;

// ── Concrete implementations ──────────────────────────────────────────────────
// Add a module here for each new provider.  The registry.rs match arms
// reference these paths directly.

pub mod africas_talking;
pub mod mpesa;
pub mod sendgrid;
pub mod twilio;

pub use registry::ProviderRegistry;
pub use traits::{
    EmailMessage, EmailProvider, PaymentProvider, PaymentStatus, SmsProvider, StkRequest,
    StkResponse, StorageProvider,
};
