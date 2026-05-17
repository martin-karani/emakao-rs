//
// Pure domain types for the billing automation scheduler.
// No I/O, no framework dependencies — plain data + enums.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

// ── Idempotency records (mirrors the three scheduler tables) ──────────────────

/// One row in `billing_cycles` — written when a rent charge is posted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingCycle {
    pub id: Uuid,
    pub agreement_id: Uuid,
    pub period_start: Date,
    pub period_end: Date,
    pub amount_kes: Decimal,
    /// Soft back-reference to ledger_entries.id (not enforced by FK).
    pub ledger_entry_id: Option<Uuid>,
    pub charged_at: OffsetDateTime,
}

/// One row in `late_fee_charges` — written when a late-fee is applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LateFeeCharge {
    pub id: Uuid,
    pub agreement_id: Uuid,
    pub period_start: Date,
    pub amount_kes: Decimal,
    pub ledger_entry_id: Option<Uuid>,
    pub charged_at: OffsetDateTime,
}

/// One row in `rent_reminders_sent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentReminderSent {
    pub id: Uuid,
    pub agreement_id: Uuid,
    pub period_start: Date,
    pub channel: ReminderChannel,
    pub sent_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderChannel {
    Email,
    Sms,
}

impl ReminderChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Sms => "sms",
        }
    }
}

// ── Scheduler commands ────────────────────────────────────────────────────────

/// Everything the scheduler needs about an active agreement to decide what
/// to do. Assembled in the repository via a single JOIN query.
#[derive(Debug, Clone)]
pub struct ActiveAgreementBillingView {
    pub agreement_id: Uuid,
    pub agency_id: Uuid,
    pub schema_name: String,
    pub unit_id: Uuid,
    pub resident_id: Uuid,
    pub rent_amount_kes: Decimal,
    pub billing_frequency: crate::domain::enums::BillingFrequency,
    pub start_date: Date,
    /// None for open-ended agreements.
    pub end_date: Option<Date>,
    /// Resident's email — for reminder/invoice dispatch.
    pub resident_email: Option<String>,
    /// Resident's phone — for SMS reminder dispatch.
    pub resident_phone: Option<String>,
    pub resident_first_name: String,
    /// Human-readable unit ref, e.g. "A3".
    pub unit_number: String,
}

/// Late-fee configuration stored per-agency or falling back to a platform default.
#[derive(Debug, Clone)]
pub struct LateFeePolicy {
    /// How many calendar days after the period_start before a late fee applies.
    pub grace_period_days: i64,
    /// Fixed amount in KES (takes precedence over `rate_percent` when set).
    pub flat_amount_kes: Option<Decimal>,
    /// Percentage of rent amount (used when `flat_amount_kes` is None).
    pub rate_percent: Option<Decimal>,
}

impl LateFeePolicy {
    /// Calculate the late fee amount for a given rent amount.
    pub fn compute(&self, rent_kes: Decimal) -> Decimal {
        if let Some(flat) = self.flat_amount_kes {
            flat
        } else if let Some(pct) = self.rate_percent {
            rent_kes * pct / Decimal::ONE_HUNDRED
        } else {
            Decimal::ZERO
        }
    }
}

// ── Period calculation helpers ────────────────────────────────────────────────

use crate::domain::enums::BillingFrequency;

/// Given the agreement's `start_date` and the billing frequency, compute the
/// `(period_start, period_end)` pair that *should* be charged as of `as_of`.
///
/// Returns `None` when the agreement hasn't started yet or is one-time and
/// past the start date.
pub fn current_billing_period(
    start_date: Date,
    frequency: BillingFrequency,
    as_of: Date,
) -> Option<(Date, Date)> {
    if as_of < start_date {
        return None;
    }

    let period_start = match frequency {
        BillingFrequency::Daily => as_of,
        BillingFrequency::Weekly => {
            // Align to the same weekday as start_date.
            let days_since = (as_of - start_date).whole_days();
            let week_offset = days_since / 7;
            start_date + time::Duration::weeks(week_offset)
        }
        BillingFrequency::Monthly => {
            // Same day-of-month as start_date in the current month.
            let mut ps = start_date;
            while ps + time::Duration::days(31) <= as_of {
                ps = advance_one_month(ps);
            }
            if ps > as_of {
                ps = recede_one_month(ps);
            }
            ps
        }
        BillingFrequency::Quarterly => advance_n_months(start_date, as_of, 3),
        BillingFrequency::SemiAnnual => advance_n_months(start_date, as_of, 6),
        BillingFrequency::Annual => advance_n_months(start_date, as_of, 12),
        BillingFrequency::OneTime => {
            if as_of == start_date {
                start_date
            } else {
                return None;
            }
        }
    };

    let period_end = match frequency {
        BillingFrequency::Daily => period_start,
        BillingFrequency::Weekly => period_start + time::Duration::days(6),
        BillingFrequency::Monthly => {
            let next = advance_one_month(period_start);
            next - time::Duration::days(1)
        }
        BillingFrequency::Quarterly => {
            let next = advance_months(period_start, 3);
            next - time::Duration::days(1)
        }
        BillingFrequency::SemiAnnual => {
            let next = advance_months(period_start, 6);
            next - time::Duration::days(1)
        }
        BillingFrequency::Annual => {
            let next = advance_months(period_start, 12);
            next - time::Duration::days(1)
        }
        BillingFrequency::OneTime => period_start,
    };

    Some((period_start, period_end))
}

// ── Internal date arithmetic ──────────────────────────────────────────────────

fn advance_one_month(d: Date) -> Date {
    advance_months(d, 1)
}

fn recede_one_month(d: Date) -> Date {
    let m = d.month();
    let y = d.year();
    let (prev_year, prev_month) = if m as u8 == 1 {
        (y - 1, time::Month::December)
    } else {
        (y, time::Month::try_from(m as u8 - 1).unwrap())
    };
    let max_day = days_in_month(prev_year, prev_month);
    let day = d.day().min(max_day);
    Date::from_calendar_date(prev_year, prev_month, day).unwrap()
}

fn advance_months(d: Date, months: i32) -> Date {
    let total_months = d.month() as i32 + months - 1;
    let year = d.year() + total_months / 12;
    let month_num = ((total_months % 12) + 1) as u8;
    let month = time::Month::try_from(month_num).unwrap();
    let max_day = days_in_month(year, month);
    let day = d.day().min(max_day);
    Date::from_calendar_date(year, month, day).unwrap()
}

fn advance_n_months(start: Date, as_of: Date, interval: i32) -> Date {
    let mut ps = start;
    loop {
        let next = advance_months(ps, interval);
        if next > as_of {
            break;
        }
        ps = next;
    }
    ps
}

fn days_in_month(year: i32, month: time::Month) -> u8 {
    match month {
        time::Month::January
        | time::Month::March
        | time::Month::May
        | time::Month::July
        | time::Month::August
        | time::Month::October
        | time::Month::December => 31,
        time::Month::April | time::Month::June | time::Month::September | time::Month::November => {
            30
        }
        time::Month::February => {
            if year % 400 == 0 || (year % 100 != 0 && year % 4 == 0) {
                29
            } else {
                28
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn monthly_period_first_of_month() {
        let start = date!(2024 - 01 - 01);
        let as_of = date!(2024 - 03 - 01);
        let (ps, pe) = current_billing_period(start, BillingFrequency::Monthly, as_of).unwrap();
        assert_eq!(ps, date!(2024 - 03 - 01));
        assert_eq!(pe, date!(2024 - 03 - 31));
    }

    #[test]
    fn monthly_period_mid_month() {
        let start = date!(2024 - 01 - 15);
        let as_of = date!(2024 - 03 - 15);
        let (ps, pe) = current_billing_period(start, BillingFrequency::Monthly, as_of).unwrap();
        assert_eq!(ps, date!(2024 - 03 - 15));
        assert_eq!(pe, date!(2024 - 04 - 14));
    }

    #[test]
    fn not_started_yet_returns_none() {
        let start = date!(2024 - 06 - 01);
        let as_of = date!(2024 - 05 - 01);
        assert!(current_billing_period(start, BillingFrequency::Monthly, as_of).is_none());
    }
}
