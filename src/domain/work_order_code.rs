//! Work order code utilities.
//!
//! Generates and validates human-readable work order codes of the form
//! `{PREFIX}-{NUMBER}`, e.g. `MGRD-0042`.
//!
//! The prefix is derived from the property name and stored on the `properties`
//! row. The number is an atomically-incremented counter on the same row.

use std::collections::HashSet;

/// Generate a candidate prefix from a property name.
///
/// Rules (in order of preference):
/// 1. If the name has 2+ words → uppercase acronym of first letters, up to 6 chars.
/// 2. If the name is a single word → first 5 uppercase alphabetic chars.
/// 3. Minimum 2 chars; padded with 'X' if the name is too short.
///
/// The returned string is always uppercase ASCII, 2–6 chars.
/// Use [`unique_prefix`] to resolve collisions before storing.
///
/// ```
/// use emakao::domain::work_order_code::candidate_prefix;
/// assert_eq!(candidate_prefix("Maple Gardens"),            "MG");
/// assert_eq!(candidate_prefix("Maple Gardens Residences"), "MGR");
/// assert_eq!(candidate_prefix("Sunrise Apartments"),       "SA");
/// assert_eq!(candidate_prefix("Parklands"),                "PARKL");
/// assert_eq!(candidate_prefix("The Grand Estate"),         "TGE");
/// assert_eq!(candidate_prefix("ABC"),                      "ABC");
/// assert_eq!(candidate_prefix("A"),                        "AX");
/// ```
pub fn candidate_prefix(name: &str) -> String {
    // Split on any non-alphanumeric character (spaces, hyphens, apostrophes…)
    let words: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    let raw = if words.len() >= 2 {
        // Multi-word: first letter of each word (ignore stop-words for very long names)
        words
            .iter()
            .filter_map(|w| w.chars().next())
            .take(6)
            .map(|c| c.to_ascii_uppercase())
            .collect::<String>()
    } else {
        // Single word: first 5 alphabetic characters
        name.chars()
            .filter(|c| c.is_alphabetic())
            .take(5)
            .map(|c| c.to_ascii_uppercase())
            .collect::<String>()
    };

    // Ensure minimum length of 2
    match raw.len() {
        0 => "WO".to_string(),
        1 => format!("{}X", raw),
        _ => raw,
    }
}

/// Given a base prefix and a set of already-taken prefixes, returns a unique
/// variant by appending a numeric suffix when needed.
///
/// ```
/// use emakao::domain::work_order_code::unique_prefix;
/// use std::collections::HashSet;
///
/// let taken: HashSet<String> = ["SA".to_string(), "SA2".to_string()].into();
/// assert_eq!(unique_prefix("SA", &taken), "SA3");
///
/// let empty: HashSet<String> = HashSet::new();
/// assert_eq!(unique_prefix("MG", &empty), "MG");
/// ```
pub fn unique_prefix(base: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base) {
        return base.to_string();
    }
    for i in 2u32..=999 {
        let candidate = format!("{}{}", base, i);
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    // Practically unreachable — an agency won't have 999 properties
    // with the same two-letter prefix. Fallback anyway.
    format!(
        "{}{}",
        &base[..base.len().min(4)],
        uuid::Uuid::new_v4().simple().to_string()[..4].to_uppercase()
    )
}

/// Build the final work order code string from a prefix and sequence number.
///
/// Numbers are zero-padded to 4 digits but grow naturally beyond 9999.
///
/// ```
/// use emakao::domain::work_order_code::format_code;
/// assert_eq!(format_code("MGRD", 1),     "MGRD-0001");
/// assert_eq!(format_code("SA",   42),    "SA-0042");
/// assert_eq!(format_code("PARKL", 9999), "PARKL-9999");
/// assert_eq!(format_code("MG",   10001), "MG-10001");
/// ```
pub fn format_code(prefix: &str, seq: i32) -> String {
    format!("{}-{:04}", prefix, seq)
}

/// Parse a code string back into (prefix, number).
/// Returns None if the format is invalid.
///
/// ```
/// use emakao::domain::work_order_code::parse_code;
/// assert_eq!(parse_code("MGRD-0042"), Some(("MGRD".to_string(), 42)));
/// assert_eq!(parse_code("invalid"),   None);
/// ```
pub fn parse_code(code: &str) -> Option<(String, i32)> {
    let (prefix, num_str) = code.split_once('-')?;
    if prefix.is_empty() || num_str.is_empty() {
        return None;
    }
    let num = num_str.parse::<i32>().ok()?;
    Some((prefix.to_string(), num))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_multi_word() {
        assert_eq!(candidate_prefix("Maple Gardens"), "MG");
        assert_eq!(candidate_prefix("The Grand Estate"), "TGE");
        assert_eq!(candidate_prefix("Sunrise View Apartments"), "SVA");
    }

    #[test]
    fn prefix_single_word() {
        assert_eq!(candidate_prefix("Parklands"), "PARKL");
        assert_eq!(candidate_prefix("Runda"), "RUNDA");
        assert_eq!(candidate_prefix("AB"), "AB");
        assert_eq!(candidate_prefix("A"), "AX");
        assert_eq!(candidate_prefix(""), "WO");
    }

    #[test]
    fn prefix_collision_resolution() {
        let taken: HashSet<String> = ["MG".into(), "MG2".into(), "MG3".into()].into();
        assert_eq!(unique_prefix("MG", &taken), "MG4");
    }

    #[test]
    fn code_formatting() {
        assert_eq!(format_code("MGRD", 1), "MGRD-0001");
        assert_eq!(format_code("MG", 10001), "MG-10001");
    }

    #[test]
    fn code_parsing() {
        assert_eq!(parse_code("MGRD-0042"), Some(("MGRD".to_string(), 42)));
        assert_eq!(parse_code("bad"), None);
    }
}
