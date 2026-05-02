/// Generates a cryptographically random one-time token for password resets.
pub fn generate_reset_token() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}
