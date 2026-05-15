// src/config/auth_config.rs
// JWT authentication configuration

use std::env;

/// Authentication configuration for JWT tokens and SIWE
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT signing secret (required in production)
    pub jwt_secret: String,
    /// Token expiration in days (default: 7)
    pub jwt_expiration_days: i64,
    /// Domain shown in SIWE message (default: app.kultgames.io)
    pub siwe_domain: String,
    /// URI shown in SIWE message (default: https://app.kultgames.io)
    pub siwe_uri: String,
    /// EVM chain ID for SIWE (default: 1 = Ethereum mainnet)
    pub siwe_chain_id: u64,
    /// Privy app ID used as JWT audience for identity-token based login.
    pub privy_app_id: Option<String>,
    /// Privy identity token verification key in JWK JSON format.
    pub privy_verification_key_jwk: Option<String>,
    /// Privy identity token verification key in PEM format.
    pub privy_verification_key_pem: Option<String>,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        Self {
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-me".to_string()),
            jwt_expiration_days: env::var("JWT_EXPIRATION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7),
            siwe_domain: env::var("SIWE_DOMAIN").unwrap_or_else(|_| "app.kultgames.io".to_string()),
            siwe_uri: env::var("SIWE_URI")
                .unwrap_or_else(|_| "https://app.kultgames.io".to_string()),
            siwe_chain_id: env::var("SIWE_CHAIN_ID")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            privy_app_id: optional_env("PRIVY_APP_ID"),
            privy_verification_key_jwk: optional_env("PRIVY_VERIFICATION_KEY_JWK"),
            privy_verification_key_pem: optional_env("PRIVY_VERIFICATION_KEY_PEM"),
        }
    }
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}
