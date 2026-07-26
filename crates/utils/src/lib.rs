use rand::{distributions::Alphanumeric, Rng};
use uuid::Uuid;

/// Generates a secure, cryptographically random token of a given length
pub fn generate_random_token(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Generates a new UUID v7 (time-ordered UUID)
pub fn generate_uuid_v7() -> Uuid {
    Uuid::now_v7()
}

/// Simple email validation helper to ensure basic formatting is correct
pub fn is_valid_email(email: &str) -> bool {
    let email_regex = r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$";
    let re = regex::Regex::new(email_regex).unwrap_or_else(|_| regex::Regex::new(".*").unwrap());
    re.is_match(email)
}
