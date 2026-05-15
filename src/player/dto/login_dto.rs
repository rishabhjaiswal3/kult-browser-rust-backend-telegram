// src/player/model/login_dto.rs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Payload for registering or logging in via SIWE (EIP-4361).
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// The Ethereum wallet address (must match the signer of `message`)
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,

    /// The full SIWE message that was signed (EIP-4361 format)
    pub message: String,

    /// The hex-encoded secp256k1 signature produced by `personal_sign`
    pub signature: String,

    /// Optional initial display name
    pub name: Option<String>,

    /// Optional arbitrary metadata (e.g. avatar)
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,

    /// Optional referral code provided by the frontend if they clicked an invite link
    #[serde(default)]
    #[serde(rename = "referralCode")]
    pub referral_code: Option<String>,
}

/// Payload for logging in with a Privy identity token and linked TON wallet.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PrivyLoginRequest {
    /// Privy identity token. Must include linked_accounts and be signed by Privy.
    #[serde(rename = "identityToken")]
    pub identity_token: String,

    /// TON wallet address that must be present in the verified Privy linked_accounts.
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,

    /// Optional initial display name
    pub name: Option<String>,

    /// Optional arbitrary metadata (e.g. avatar)
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,

    /// Optional referral code provided by the frontend if they clicked an invite link
    #[serde(default)]
    #[serde(rename = "referralCode")]
    pub referral_code: Option<String>,
}

/// GET /api/player/nonce - Response body
#[derive(Debug, Serialize, ToSchema)]
pub struct NonceResponse {
    pub nonce: String,
}

/// POST /api/player/login - Response body
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub player: PlayerInfo,
}

/// Minimal player info returned on login
#[derive(Debug, Serialize, ToSchema)]
pub struct PlayerInfo {
    pub id: String,

    #[serde(rename = "walletAddress")]
    pub wallet_address: String,

    pub name: String,
}
