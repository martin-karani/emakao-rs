use rand::Rng;
use uuid::Uuid;

/// Generate a random 32‑character token (UUID v4 without dashes)
pub fn generate_token() -> String {
    Uuid::new_v4().to_string().replace('-', "")
}

/// Generate an 8‑character alphanumeric temporary password (no ambiguous chars).
/// Suitable for SMS delivery.
pub fn generate_temp_password() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}
