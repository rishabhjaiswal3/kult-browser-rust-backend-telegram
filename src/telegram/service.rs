use crate::config::CONFIG;
use crate::handler::AppError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct TelegramBotService {
    client: Client,
}

impl TelegramBotService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn handle_update(&self, update: TelegramUpdate) -> Result<(), AppError> {
        let Some(message) = update.message else {
            return Ok(());
        };

        let text = message.text.unwrap_or_default();
        if text.starts_with("/start") || text.starts_with("/app") {
            self.send_mini_app_button(message.chat.id).await?;
        }

        Ok(())
    }

    pub async fn send_mini_app_button(&self, chat_id: i64) -> Result<(), AppError> {
        let mini_app_url = CONFIG.telegram.mini_app_url.clone();
        let payload = json!({
            "chat_id": chat_id,
            "text": "Open Kult Games inside Telegram.",
            "reply_markup": {
                "inline_keyboard": [[{
                    "text": "Open Kult Games",
                    "web_app": { "url": mini_app_url }
                }]]
            }
        });

        self.call_telegram("sendMessage", payload).await?;
        Ok(())
    }

    pub async fn set_webhook(&self) -> Result<Value, AppError> {
        let webhook_url =
            CONFIG.telegram.webhook_url.as_deref().ok_or_else(|| {
                AppError::BadRequest("TELEGRAM_WEBHOOK_URL is required".to_string())
            })?;

        let mut payload = json!({
            "url": webhook_url,
            "allowed_updates": ["message"]
        });

        if let Some(secret) = CONFIG.telegram.webhook_secret_token.as_deref() {
            payload["secret_token"] = json!(secret);
        }

        let webhook = self.call_telegram("setWebhook", payload).await?;
        let menu_button = self.set_menu_button().await?;

        Ok(json!({
            "webhook": webhook,
            "menuButton": menu_button
        }))
    }

    pub async fn set_menu_button(&self) -> Result<Value, AppError> {
        let payload = json!({
            "menu_button": {
                "type": "web_app",
                "text": "Open Kult Games",
                "web_app": {
                    "url": CONFIG.telegram.mini_app_url
                }
            }
        });

        self.call_telegram("setChatMenuButton", payload).await
    }

    pub async fn delete_webhook(&self) -> Result<Value, AppError> {
        self.call_telegram("deleteWebhook", json!({ "drop_pending_updates": true }))
            .await
    }

    async fn call_telegram(&self, method: &str, payload: Value) -> Result<Value, AppError> {
        let token = CONFIG.telegram.bot_token.as_deref().ok_or_else(|| {
            AppError::BadRequest("TELEGRAM_BOT_TOKEN is not configured".to_string())
        })?;

        let url = format!("https://api.telegram.org/bot{}/{}", token, method);
        let response = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram API request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Invalid Telegram API response: {}", e)))?;

        if !status.is_success() || body.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(AppError::BadRequest(format!(
                "Telegram API error for {}: {}",
                method, body
            )));
        }

        Ok(body)
    }
}

#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub chat: TelegramChat,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Serialize)]
pub struct TelegramStatus {
    pub enabled: bool,
    #[serde(rename = "miniAppUrl")]
    pub mini_app_url: String,
    #[serde(rename = "webhookConfigured")]
    pub webhook_configured: bool,
}
