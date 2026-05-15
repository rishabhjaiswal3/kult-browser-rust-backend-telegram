use crate::config::CONFIG;
use jsonwebtoken::{
    decode, decode_header,
    jwk::{Jwk, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct PrivyIdentityClaims {
    #[allow(dead_code)]
    sub: String,
    iss: String,
    aud: String,
    linked_accounts: Option<String>,
    #[allow(dead_code)]
    exp: usize,
    #[allow(dead_code)]
    iat: Option<usize>,
}

pub fn verify_privy_ton_wallet(identity_token: &str, wallet_address: &str) -> Result<(), String> {
    let claims = verify_identity_token(identity_token)?;
    if claims.iss != "privy.io" {
        return Err("Privy identity token has an invalid issuer".to_string());
    }

    let linked_accounts_raw = claims
        .linked_accounts
        .ok_or_else(|| "Privy identity token is missing linked_accounts".to_string())?;
    let linked_accounts: Value = serde_json::from_str(&linked_accounts_raw)
        .map_err(|e| format!("Invalid linked_accounts claim: {}", e))?;
    let accounts = linked_accounts
        .as_array()
        .ok_or_else(|| "Privy linked_accounts claim is not an array".to_string())?;

    let requested = wallet_address.trim();
    let has_ton_wallet = accounts.iter().any(|account| {
        let address = string_field(account, "address").unwrap_or_default();
        let chain_type = string_field(account, "chainType")
            .or_else(|| string_field(account, "chain_type"))
            .unwrap_or_default();
        let account_type = string_field(account, "type").unwrap_or_default();

        account_type == "wallet" && chain_type == "ton" && address == requested
    });

    if !has_ton_wallet {
        return Err("TON wallet is not linked to the verified Privy user".to_string());
    }

    Ok(())
}

fn verify_identity_token(identity_token: &str) -> Result<PrivyIdentityClaims, String> {
    let app_id = CONFIG
        .auth
        .privy_app_id
        .as_deref()
        .ok_or_else(|| "PRIVY_APP_ID is not configured".to_string())?;

    let decoding_key = decoding_key_for_token(identity_token)?;
    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_audience(&[app_id]);
    validation.set_issuer(&["privy.io"]);

    let token = decode::<PrivyIdentityClaims>(identity_token, &decoding_key, &validation)
        .map_err(|e| format!("Invalid Privy identity token: {}", e))?;

    if token.claims.aud != app_id {
        return Err("Privy identity token has an invalid audience".to_string());
    }

    Ok(token.claims)
}

fn decoding_key_for_token(identity_token: &str) -> Result<DecodingKey, String> {
    if let Some(jwk_json) = CONFIG.auth.privy_verification_key_jwk.as_deref() {
        return decoding_key_from_jwk_json(identity_token, jwk_json);
    }

    if let Some(pem) = CONFIG.auth.privy_verification_key_pem.as_deref() {
        return DecodingKey::from_ec_pem(pem.as_bytes())
            .map_err(|e| format!("Invalid PRIVY_VERIFICATION_KEY_PEM: {}", e));
    }

    Err("Set PRIVY_VERIFICATION_KEY_JWK or PRIVY_VERIFICATION_KEY_PEM".to_string())
}

fn decoding_key_from_jwk_json(identity_token: &str, jwk_json: &str) -> Result<DecodingKey, String> {
    let value: Value = serde_json::from_str(jwk_json)
        .map_err(|e| format!("Invalid PRIVY_VERIFICATION_KEY_JWK JSON: {}", e))?;

    if value.get("keys").is_some() {
        let header = decode_header(identity_token)
            .map_err(|e| format!("Invalid Privy identity token header: {}", e))?;
        let key_id = header
            .kid
            .ok_or_else(|| "Privy identity token is missing kid".to_string())?;
        let jwk_set: JwkSet =
            serde_json::from_value(value).map_err(|e| format!("Invalid JWK set: {}", e))?;
        let jwk = jwk_set
            .find(&key_id)
            .ok_or_else(|| format!("No Privy JWK found for kid {}", key_id))?;
        return DecodingKey::from_jwk(jwk).map_err(|e| format!("Invalid Privy JWK: {}", e));
    }

    let jwk: Jwk = serde_json::from_value(value).map_err(|e| format!("Invalid JWK: {}", e))?;
    DecodingKey::from_jwk(&jwk).map_err(|e| format!("Invalid Privy JWK: {}", e))
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_string)
}
