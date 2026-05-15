// src/config/mod.rs
// Central configuration module - loads all config from environment

pub mod app_config;
pub mod auth_config;
pub mod bd_config;
pub mod db_config;
pub mod do_config;
pub mod log_config;
pub mod onchain_config;
pub mod scrape_config;
pub mod telegram_config;
pub mod valkey_config;
pub mod zg_config;

use once_cell::sync::Lazy;

pub use app_config::AppConfig;
pub use auth_config::AuthConfig;
pub use bd_config::BdConfig;
pub use db_config::DbConfig;
pub use do_config::DoConfig;
pub use log_config::LogConfig;
pub use onchain_config::OnchainConfig;
pub use scrape_config::ScrapeConfig;
pub use telegram_config::TelegramConfig;
pub use valkey_config::ValkeyConfig;
pub use zg_config::ZgConfig;

/// Global configuration loaded once at startup
#[derive(Debug, Clone)]
pub struct Config {
    pub app: AppConfig,
    pub auth: AuthConfig,
    pub bright_data: BdConfig,
    pub db: DbConfig,
    pub do_spaces: DoConfig,
    pub log: LogConfig,
    pub onchain: OnchainConfig,
    pub scrape: ScrapeConfig,
    pub telegram: TelegramConfig,
    pub valkey: ValkeyConfig,
    pub zg: ZgConfig,
}

impl Config {
    /// Load all configuration from environment.
    /// Call this after dotenvy::dotenv() has loaded .env
    fn from_env() -> Self {
        Self {
            app: AppConfig::from_env(),
            auth: AuthConfig::from_env(),
            bright_data: BdConfig::from_env(),
            db: DbConfig::from_env(),
            do_spaces: DoConfig::from_env(),
            log: LogConfig::from_env(),
            onchain: OnchainConfig::from_env(),
            scrape: ScrapeConfig::from_env(),
            telegram: TelegramConfig::from_env(),
            valkey: ValkeyConfig::from_env(),
            zg: ZgConfig::from_env(),
        }
    }
}

/// Global config instance - access via `CONFIG.app.port`, `CONFIG.db.mongo_uri`, `CONFIG.log.level`, etc.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // Load .env file (ignore if missing - production uses real env vars)
    let _ = dotenvy::dotenv();
    Config::from_env()
});
