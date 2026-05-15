use axum::{
    extract::State,
    http::{header::HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};

use crate::config::CONFIG;
use crate::handler::{ApiResponse, AppError};
use crate::telegram::service::{TelegramBotService, TelegramStatus, TelegramUpdate};

#[derive(Clone)]
pub struct TelegramState {
    service: TelegramBotService,
}

pub fn routes() -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/webhook", post(webhook))
        .route("/webhook", delete(delete_webhook))
        .route("/webhook/set", post(set_webhook))
        .with_state(TelegramState {
            service: TelegramBotService::new(),
        })
}

async fn status() -> Response {
    ApiResponse::success(TelegramStatus {
        enabled: CONFIG.telegram.enabled(),
        mini_app_url: CONFIG.telegram.mini_app_url.clone(),
        webhook_configured: CONFIG.telegram.webhook_url.is_some(),
    })
    .into_response()
}

async fn webhook(
    State(state): State<TelegramState>,
    headers: HeaderMap,
    payload: Result<Json<TelegramUpdate>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Err(e) = verify_webhook_secret(&headers) {
        return e.into_response();
    }

    let Json(update) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return AppError::BadRequest(rejection.body_text()).into_response(),
    };

    match state.service.handle_update(update).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn set_webhook(State(state): State<TelegramState>, headers: HeaderMap) -> Response {
    if let Err(e) = verify_admin_secret(&headers) {
        return e.into_response();
    }

    match state.service.set_webhook().await {
        Ok(data) => ApiResponse::success(data).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn delete_webhook(State(state): State<TelegramState>, headers: HeaderMap) -> Response {
    if let Err(e) = verify_admin_secret(&headers) {
        return e.into_response();
    }

    match state.service.delete_webhook().await {
        Ok(data) => ApiResponse::success(data).into_response(),
        Err(e) => e.into_response(),
    }
}

fn verify_webhook_secret(headers: &HeaderMap) -> Result<(), AppError> {
    let Some(expected) = CONFIG.telegram.webhook_secret_token.as_deref() else {
        return Ok(());
    };

    let actual = headers
        .get("x-telegram-bot-api-secret-token")
        .and_then(|value| value.to_str().ok());

    if actual == Some(expected) {
        Ok(())
    } else {
        Err(AppError::Unauthorized(
            "Invalid Telegram webhook secret".to_string(),
        ))
    }
}

fn verify_admin_secret(headers: &HeaderMap) -> Result<(), AppError> {
    let Some(expected) = CONFIG.telegram.admin_secret.as_deref() else {
        return Ok(());
    };

    let actual = headers
        .get("x-telegram-admin-secret")
        .and_then(|value| value.to_str().ok());

    if actual == Some(expected) {
        Ok(())
    } else {
        Err(AppError::Unauthorized(
            "Invalid Telegram admin secret".to_string(),
        ))
    }
}
