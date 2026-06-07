use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{domain::agency::AgencySettings, presentation::app_state::AppState};

/// The typed agency-id wrapper expected in extensions.
/// Must be inserted earlier in the middleware stack (e.g. by JWT validation).
#[derive(Clone, Copy)]
pub struct AgencyId(pub uuid::Uuid);

// ── Middleware ────────────────────────────────────────────────────────────────

pub async fn inject_branding_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(AgencyId(agency_id)) = req.extensions().get::<AgencyId>().copied() {
        match state.customisation().settings.get_or_load(agency_id, state.infra.tenant_pools.platform()).await {
            Ok(settings) => {
                req.extensions_mut().insert(settings);
            }
            Err(e) => {
                tracing::error!(
                    agency = %agency_id,
                    "branding middleware: failed to load settings: {e}"
                );
                // Continue without settings — handlers that need them will
                // fail explicitly (better than a silent wrong-tenant render).
            }
        }
    }
    next.run(req).await
}

// ── PDF / email rendering helper ──────────────────────────────────────────────

/// Data passed to the invoice MiniJinja template.
pub struct InvoiceData {
    pub invoice_number: String,
    pub issue_date: String,
    pub due_date: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub subtotal_kes: rust_decimal::Decimal,
    pub vat_kes: rust_decimal::Decimal,
    pub total_kes: rust_decimal::Decimal,
    pub tenant_name: String,
    pub property_name: String,
    pub unit_ref: String,
}

pub struct InvoiceLineItem {
    pub description: String,
    pub amount_kes: rust_decimal::Decimal,
}

/// Render an invoice to HTML using the agency's custom `invoice_template`
/// (MiniJinja). Returns an error if no template is configured.
pub fn render_invoice(
    settings: &AgencySettings,
    env: &minijinja::Environment<'_>,
    data: &InvoiceData,
) -> anyhow::Result<String> {
    let tmpl_str = settings
        .branding
        .invoice_template
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("No invoice template configured for this agency"))?;

    let tmpl = env.template_from_str(tmpl_str)?;

    Ok(tmpl.render(minijinja::context! {
        branding    => serde_json::to_value(&settings.branding)?,
        invoice_number => &data.invoice_number,
        issue_date  => &data.issue_date,
        due_date    => &data.due_date,
        line_items  => data.line_items.iter().map(|li| {
            serde_json::json!({
                "description": li.description,
                "amount_kes":  li.amount_kes.to_string(),
            })
        }).collect::<Vec<_>>(),
        subtotal_kes => data.subtotal_kes.to_string(),
        vat_kes      => data.vat_kes.to_string(),
        total_kes    => data.total_kes.to_string(),
        tenant_name  => &data.tenant_name,
        property_name => &data.property_name,
        unit_ref     => &data.unit_ref,
    })?)
}
