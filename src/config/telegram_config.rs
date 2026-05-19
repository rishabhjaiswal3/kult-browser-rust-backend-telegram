use std::env;

const DEFAULT_TELEGRAM_MINI_APP_URL: &str = "https://kult-telegram-mrvv3.ondigitalocean.app/";
const DEFAULT_TELEGRAM_WEBHOOK_URL: &str =
    "https://kult-browser-rust-l2lwg.ondigitalocean.app/api/telegram/webhook";

#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: Option<String>,
    pub mini_app_url: String,
    pub webhook_url: Option<String>,
    pub webhook_secret_token: Option<String>,
    pub admin_secret: Option<String>,
}

impl TelegramConfig {
    pub fn from_env() -> Self {
        Self {
            bot_token: optional_env("TELEGRAM_BOT_TOKEN"),
            mini_app_url: env::var("TELEGRAM_MINI_APP_URL")
                .unwrap_or_else(|_| DEFAULT_TELEGRAM_MINI_APP_URL.to_string()),
            webhook_url: Some(
                env::var("TELEGRAM_WEBHOOK_URL")
                    .unwrap_or_else(|_| DEFAULT_TELEGRAM_WEBHOOK_URL.to_string()),
            ),
            webhook_secret_token: optional_env("TELEGRAM_WEBHOOK_SECRET_TOKEN"),
            admin_secret: optional_env("TELEGRAM_ADMIN_SECRET"),
        }
    }

    pub fn enabled(&self) -> bool {
        self.bot_token.is_some()
    }
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
