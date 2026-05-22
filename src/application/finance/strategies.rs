// src/application/finance/strategies.rs
//
// Strategy pattern for the two most variable financial behaviours:
//   1. Late-fee calculation  (flat | percent, with optional cap)
//   2. Payment allocation    (oldest_first | penalties_first | rent_first)
//
// Usage inside a use-case:
//
//   let policy = late_fee_repo::get_default(pool, agency_id).await?;
//   let fee    = late_fee_strategy(&policy).calculate(&ctx);
//
//   let alloc  = allocation_strategy(&settings.workflows.payment_allocation_strategy);
//   alloc.sort_charges(&mut open_charges);

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

// ── Late-fee strategies ───────────────────────────────────────────────────────

/// Context passed to every `LateFeeStrategy::calculate` call.
pub struct LateFeeContext {
    pub outstanding_kes: Decimal,
    pub rent_amount_kes: Decimal,
    pub days_overdue: i32,
}

pub trait LateFeeStrategy: Send + Sync {
    fn calculate(&self, ctx: &LateFeeContext) -> Decimal;
}

// ── Flat fee ──────────────────────────────────────────────────────────────────

pub struct FlatFeeStrategy {
    pub amount: Decimal,
    pub max: Option<Decimal>,
}

impl LateFeeStrategy for FlatFeeStrategy {
    fn calculate(&self, _ctx: &LateFeeContext) -> Decimal {
        match self.max {
            Some(cap) => self.amount.min(cap),
            None => self.amount,
        }
    }
}

// ── Percent of rent ───────────────────────────────────────────────────────────

pub struct PercentFeeStrategy {
    /// e.g. `Decimal::from_str("0.05").unwrap()` for 5 %.
    pub rate: Decimal,
    pub max: Option<Decimal>,
}

impl LateFeeStrategy for PercentFeeStrategy {
    fn calculate(&self, ctx: &LateFeeContext) -> Decimal {
        let fee = ctx.rent_amount_kes * self.rate;
        match self.max {
            Some(cap) => fee.min(cap),
            None => fee,
        }
    }
}

// ── DB row → strategy ─────────────────────────────────────────────────────────

/// Minimal projection from `late_fee_policy` — matches the SQLx query! output.
pub struct LateFeePolicy {
    pub fee_type: String,
    pub flat_amount_kes: Option<Decimal>,
    pub percent_of_rent: Option<Decimal>,
    pub max_fee_kes: Option<Decimal>,
}

pub fn late_fee_strategy(p: &LateFeePolicy) -> Box<dyn LateFeeStrategy> {
    match p.fee_type.as_str() {
        "percent" => Box::new(PercentFeeStrategy {
            rate: p.percent_of_rent.unwrap_or_default(),
            max: p.max_fee_kes,
        }),
        _ => Box::new(FlatFeeStrategy {
            amount: p.flat_amount_kes.unwrap_or_default(),
            max: p.max_fee_kes,
        }),
    }
}

// ── Payment allocation strategies ────────────────────────────────────────────

/// A line item on the tenant ledger that is awaiting payment.
#[derive(Debug, Clone)]
pub struct LedgerCharge {
    pub id: uuid::Uuid,
    pub entry_type: String, // "rent" | "late_fee" | "penalty" | "deposit" | …
    pub amount_kes: Decimal,
    pub posted_at: DateTime<Utc>,
}

pub trait AllocationStrategy: Send + Sync {
    /// Sort `charges` in the order they should be paid off (index 0 = first).
    fn sort_charges(&self, charges: &mut Vec<LedgerCharge>);
}

// ── Oldest-first (default) ────────────────────────────────────────────────────

pub struct OldestFirstStrategy;

impl AllocationStrategy for OldestFirstStrategy {
    fn sort_charges(&self, charges: &mut Vec<LedgerCharge>) {
        charges.sort_by_key(|c| c.posted_at);
    }
}

// ── Penalties first, then oldest ─────────────────────────────────────────────

pub struct PenaltiesFirstStrategy;

impl AllocationStrategy for PenaltiesFirstStrategy {
    fn sort_charges(&self, charges: &mut Vec<LedgerCharge>) {
        charges.sort_by(|a, b| {
            let a_pen = matches!(a.entry_type.as_str(), "late_fee" | "penalty");
            let b_pen = matches!(b.entry_type.as_str(), "late_fee" | "penalty");
            // penalties first (true > false), then oldest within each group
            b_pen.cmp(&a_pen).then(a.posted_at.cmp(&b.posted_at))
        });
    }
}

// ── Rent first, then oldest ───────────────────────────────────────────────────

pub struct RentFirstStrategy;

impl AllocationStrategy for RentFirstStrategy {
    fn sort_charges(&self, charges: &mut Vec<LedgerCharge>) {
        charges.sort_by(|a, b| {
            let a_rent = a.entry_type == "rent";
            let b_rent = b.entry_type == "rent";
            b_rent.cmp(&a_rent).then(a.posted_at.cmp(&b.posted_at))
        });
    }
}

// ── String → strategy ────────────────────────────────────────────────────────

pub fn allocation_strategy(s: &str) -> Box<dyn AllocationStrategy> {
    match s {
        "penalties_first" => Box::new(PenaltiesFirstStrategy),
        "rent_first" => Box::new(RentFirstStrategy),
        _ => Box::new(OldestFirstStrategy),
    }
}
