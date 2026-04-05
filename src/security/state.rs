use crate::security::jwt::Keys;
use jsonwebtoken::{DecodingKey, EncodingKey};

pub struct SecurityState {
    pub keys: Keys,
}

impl SecurityState {
    pub fn new() -> Self {
        let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let keys = Keys::new(
            EncodingKey::from_secret(secret.as_bytes()),
            DecodingKey::from_secret(secret.as_bytes()),
        );
        Self { keys }
    }
}
