use crate::handler::{ApiResponse, AppError};
use crate::middleware::AuthPlayer;
use crate::player::dto::{LoginRequest, PrivyLoginRequest, UpdateNameRequest};
use crate::player::PlayerService;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

/// Shared state for player endpoints
#[derive(Clone)]
pub struct PlayerState {
    pub player_service: PlayerService,
}

#[derive(Deserialize)]
pub struct NonceQuery {
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,
}

/// GET /api/player/nonce
#[utoipa::path(
    get,
    path = "/api/player/nonce",
    params(
        ("walletAddress" = String, Query, description = "Ethereum wallet address requesting a nonce")
    ),
    responses(
        (status = 200, description = "One-time SIWE nonce issued", body = crate::openapi::NonceApiResponse),
        (status = 400, description = "Missing walletAddress query param", body = crate::openapi::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::openapi::ErrorResponse)
    ),
    tag = "Player"
)]
pub async fn get_nonce(
    State(state): State<PlayerState>,
    Query(params): Query<NonceQuery>,
) -> Response {
    if params.wallet_address.is_empty() {
        return AppError::BadRequest("walletAddress query param is required".to_string())
            .into_response();
    }
    match state.player_service.get_nonce(&params.wallet_address).await {
        Ok(data) => ApiResponse::success(data).into_response(),
        Err(e) => e.into_response(),
    }
}

/// POST /api/player/login
#[utoipa::path(
    post,
    path = "/api/player/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login or create a player", body = crate::openapi::LoginApiResponse),
        (status = 400, description = "Invalid login payload", body = crate::openapi::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::openapi::ErrorResponse)
    ),
    tag = "Player"
)]
pub async fn login(
    State(state): State<PlayerState>,
    headers: axum::http::header::HeaderMap,
    payload: Result<Json<LoginRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(p) => p,
        Err(rejection) => return AppError::BadRequest(rejection.body_text()).into_response(),
    };

    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("0.0.0.0");

    match state.player_service.login(request, ip_address).await {
        Ok(data) => ApiResponse::success(data).into_response(),
        Err(e) => e.into_response(),
    }
}

/// POST /api/player/privy-login
#[utoipa::path(
    post,
    path = "/api/player/privy-login",
    request_body = PrivyLoginRequest,
    responses(
        (status = 200, description = "Login or create a player from verified Privy TON identity", body = crate::openapi::LoginApiResponse),
        (status = 400, description = "Invalid login payload", body = crate::openapi::ErrorResponse),
        (status = 401, description = "Invalid Privy identity token", body = crate::openapi::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::openapi::ErrorResponse)
    ),
    tag = "Player"
)]
pub async fn privy_login(
    State(state): State<PlayerState>,
    headers: axum::http::header::HeaderMap,
    payload: Result<Json<PrivyLoginRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(p) => p,
        Err(rejection) => return AppError::BadRequest(rejection.body_text()).into_response(),
    };

    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("0.0.0.0");

    match state.player_service.privy_login(request, ip_address).await {
        Ok(data) => ApiResponse::success(data).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/player/profile
#[utoipa::path(
    get,
    path = "/api/player/profile",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Authenticated player profile", body = crate::openapi::PlayerProfileApiResponse),
        (status = 401, description = "Missing or invalid bearer token", body = crate::openapi::ErrorResponse),
        (status = 404, description = "Player not found", body = crate::openapi::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::openapi::ErrorResponse)
    ),
    tag = "Player"
)]
pub async fn get_profile(State(state): State<PlayerState>, auth: AuthPlayer) -> Response {
    match state.player_service.get_profile(&auth.wallet_address).await {
        Ok(data) => ApiResponse::success(data).into_response(),
        Err(e) => e.into_response(),
    }
}

/// PATCH /api/player/name
#[utoipa::path(
    patch,
    path = "/api/player/name",
    security(
        ("bearer_auth" = [])
    ),
    request_body = UpdateNameRequest,
    responses(
        (status = 200, description = "Updated player display name", body = crate::openapi::UpdateNameApiResponse),
        (status = 400, description = "Invalid update payload", body = crate::openapi::ErrorResponse),
        (status = 401, description = "Missing or invalid bearer token", body = crate::openapi::ErrorResponse),
        (status = 404, description = "Player not found", body = crate::openapi::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::openapi::ErrorResponse)
    ),
    tag = "Player"
)]
pub async fn update_name(
    State(state): State<PlayerState>,
    auth: AuthPlayer,
    payload: Result<Json<UpdateNameRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(p) => p,
        Err(rejection) => return AppError::BadRequest(rejection.body_text()).into_response(),
    };

    match state
        .player_service
        .update_name(&auth.wallet_address, request)
        .await
    {
        Ok(data) => ApiResponse::success(data).into_response(),
        Err(e) => e.into_response(),
    }
}
