use crate::handler::AppError;
use crate::leaderboard::repository::GlobalLeaderboardRepository;
use crate::leaderboard::service::GameLeaderboardService;
use crate::middleware::AuthService;
use crate::player::dto::{
    GameScoreEntry, LoginRequest, LoginResponse, NonceResponse, PlayerInfo, PlayerProfile,
    PlayerProfileResponse, PrivyLoginRequest, TelegramMiniAppLoginRequest, UpdateNameRequest,
    UpdateNameResponse,
};
use crate::player::privy::verify_privy_ton_wallet;
use crate::player::telegram::verify_telegram_init_data;
use crate::player::repository::PlayerRepository;
use crate::player::siwe::{extract_nonce, verify_wallet_signature, NonceRepository};
use mongodb::bson::Document;
use serde_json::Value;

use crate::agent::repository::agent_repository::AgentRepository;

use crate::referral::anti_fraud::AntiFraudService;

/// Service layer for Player operations.
#[derive(Clone)]
pub struct PlayerService {
    player_repo: PlayerRepository,
    global_lb_repo: GlobalLeaderboardRepository,
    game_lb_service: GameLeaderboardService,
    agent_repo: AgentRepository,
    anti_fraud_service: Option<std::sync::Arc<AntiFraudService>>,
    nonce_repo: NonceRepository,
}

impl PlayerService {
    pub fn new(
        player_repo: PlayerRepository,
        global_lb_repo: GlobalLeaderboardRepository,
        game_lb_service: GameLeaderboardService,
        agent_repo: AgentRepository,
        anti_fraud_service: Option<std::sync::Arc<AntiFraudService>>,
        nonce_repo: NonceRepository,
    ) -> Self {
        Self {
            player_repo,
            global_lb_repo,
            game_lb_service,
            agent_repo,
            anti_fraud_service,
            nonce_repo,
        }
    }

    /// Issue a one-time SIWE nonce for the given wallet address.
    pub async fn get_nonce(&self, wallet: &str) -> Result<NonceResponse, AppError> {
        let nonce = nanoid::nanoid!(16);
        self.nonce_repo
            .create_nonce(wallet, &nonce)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, wallet = %wallet, "Failed to store SIWE nonce");
                AppError::Internal(e.to_string())
            })?;
        Ok(NonceResponse { nonce })
    }

    /// Handle player login (find or create) — requires SIWE signature verification.
    pub async fn login(
        &self,
        request: LoginRequest,
        ip_address: &str,
    ) -> Result<LoginResponse, AppError> {
        let wallet = request.wallet_address.trim().to_string();
        tracing::info!(wallet = %wallet, "Player login attempt");

        if wallet.is_empty() {
            return Err(AppError::BadRequest(
                "walletAddress is required".to_string(),
            ));
        }

        // --- SIWE verification ---
        // 1. Verify the signature recovers to the claimed wallet
        if let Err(reason) = verify_wallet_signature(&wallet, &request.message, &request.signature)
        {
            tracing::warn!(wallet = %wallet, reason = %reason, "SIWE signature verification failed");
            return Err(AppError::Unauthorized(
                "Invalid signature — wallet ownership not proven".to_string(),
            ));
        }

        // 2. Extract nonce from the signed message and verify it was issued by us
        let nonce = extract_nonce(&request.message).ok_or_else(|| {
            tracing::warn!(wallet = %wallet, "SIWE message missing Nonce field");
            AppError::BadRequest("Invalid SIWE message: missing Nonce field".to_string())
        })?;

        let nonce_valid = self
            .nonce_repo
            .consume_nonce(&wallet, nonce)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, wallet = %wallet, "DB error consuming SIWE nonce");
                AppError::Internal(e.to_string())
            })?;

        if !nonce_valid {
            tracing::warn!(wallet = %wallet, nonce = %nonce, "SIWE nonce not found or already used");
            return Err(AppError::Unauthorized(
                "Nonce invalid or expired — request a new nonce and sign again".to_string(),
            ));
        }

        tracing::info!(wallet = %wallet, "SIWE verification passed");

        let name = request.name.unwrap_or_else(|| {
            let suffix = format!("{:x}", chrono::Utc::now().timestamp_millis())
                .chars()
                .rev()
                .take(8)
                .collect::<String>();
            format!("kult-player_{}", suffix)
        });

        let metadata: Option<Document> = request
            .metadata
            .and_then(|v| mongodb::bson::to_document(&v).ok());

        let (player, is_new) = self
            .player_repo
            .find_or_create(&wallet, &name, metadata)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, wallet = %wallet, "DB error during login");
                AppError::Internal(e)
            })?;

        if is_new {
            tracing::info!(wallet = %wallet, name = %name, "New player registered");

            // Process referral if one was provided
            if let Some(ref_code) = request.referral_code {
                if let Some(fraud_service) = &self.anti_fraud_service {
                    let player_id_str = player
                        .id
                        .map(|oid| oid.to_hex())
                        .unwrap_or_else(|| wallet.clone());

                    if let Err(e) = fraud_service
                        .process_referral_signup(&player_id_str, &ref_code, ip_address)
                        .await
                    {
                        tracing::error!(
                            error = %e,
                            wallet = %wallet,
                            "Failed to process referral signup"
                        );
                    } else {
                        tracing::info!(wallet = %wallet, code = %ref_code, "Referral pushed to validation queue");
                    }
                }
            }

            // Automatically generate a Web3 AI Agent identity for this new user
            if let Err(e) = self.agent_repo.create_agent_for_new_user(&wallet).await {
                tracing::error!(
                    error = %e,
                    wallet = %wallet,
                    "Failed to generate AI agent for new user during login registration"
                );
                // We don't necessarily want to block the user from logging in if agent gen fails,
                // but we must log it as a critical error.
            }
        } else {
            tracing::debug!(wallet = %wallet, "Existing player logged in");
        }

        let token = AuthService::sign_token(&player).map_err(|e| {
            tracing::error!(error = %e, "Failed to sign JWT token");
            AppError::Internal(e)
        })?;

        Ok(LoginResponse {
            token,
            player: PlayerInfo {
                id: player.id.map(|oid| oid.to_hex()).unwrap_or_default(),
                wallet_address: player.wallet_address,
                name: player.name,
            },
        })
    }

    /// Handle player login via Privy identity token with a linked TON wallet.
    pub async fn privy_login(
        &self,
        request: PrivyLoginRequest,
        ip_address: &str,
    ) -> Result<LoginResponse, AppError> {
        let wallet = request.wallet_address.trim().to_string();
        tracing::info!(wallet = %wallet, "Privy TON login attempt");

        if wallet.is_empty() {
            return Err(AppError::BadRequest(
                "walletAddress is required".to_string(),
            ));
        }

        verify_privy_ton_wallet(&request.identity_token, &wallet).map_err(|reason| {
            tracing::warn!(wallet = %wallet, reason = %reason, "Privy TON verification failed");
            AppError::Unauthorized(reason)
        })?;

        tracing::info!(wallet = %wallet, "Privy TON verification passed");

        let name = request.name.unwrap_or_else(|| {
            let suffix = format!("{:x}", chrono::Utc::now().timestamp_millis())
                .chars()
                .rev()
                .take(8)
                .collect::<String>();
            format!("kult-player_{}", suffix)
        });

        let metadata: Option<Document> = request
            .metadata
            .and_then(|v| mongodb::bson::to_document(&v).ok());

        let (player, is_new) = self
            .player_repo
            .find_or_create(&wallet, &name, metadata)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, wallet = %wallet, "DB error during Privy TON login");
                AppError::Internal(e)
            })?;

        if is_new {
            tracing::info!(wallet = %wallet, name = %name, "New TON player registered");

            if let Some(ref_code) = request.referral_code {
                if let Some(fraud_service) = &self.anti_fraud_service {
                    let player_id_str = player
                        .id
                        .map(|oid| oid.to_hex())
                        .unwrap_or_else(|| wallet.clone());

                    if let Err(e) = fraud_service
                        .process_referral_signup(&player_id_str, &ref_code, ip_address)
                        .await
                    {
                        tracing::error!(
                            error = %e,
                            wallet = %wallet,
                            "Failed to process referral signup"
                        );
                    }
                }
            }
        }

        let token = AuthService::sign_token(&player).map_err(|e| {
            tracing::error!(error = %e, "Failed to sign JWT token");
            AppError::Internal(e)
        })?;

        Ok(LoginResponse {
            token,
            player: PlayerInfo {
                id: player.id.map(|oid| oid.to_hex()).unwrap_or_default(),
                wallet_address: player.wallet_address,
                name: player.name,
            },
        })
    }

    /// Handle player login from a Telegram Mini App using raw `initData`.
    ///
    /// Verifies the HMAC-SHA256 signature against `TELEGRAM_BOT_TOKEN`,
    /// then finds or creates a player keyed by `tg:{telegram_user_id}`.
    pub async fn telegram_miniapp_login(
        &self,
        request: TelegramMiniAppLoginRequest,
        ip_address: &str,
    ) -> Result<LoginResponse, AppError> {
        let bot_token = crate::config::CONFIG
            .telegram
            .bot_token
            .as_deref()
            .ok_or_else(|| {
                AppError::Internal("TELEGRAM_BOT_TOKEN is not configured on the server".to_string())
            })?;

        let tg_user = verify_telegram_init_data(&request.init_data, bot_token).map_err(|reason| {
            tracing::warn!(reason = %reason, "Telegram Mini App initData verification failed");
            AppError::Unauthorized(reason)
        })?;

        let wallet = format!("tg:{}", tg_user.id);
        tracing::info!(wallet = %wallet, telegram_id = tg_user.id, "Telegram Mini App login");

        let display_name = request.name.unwrap_or_else(|| {
            tg_user.username.clone().unwrap_or_else(|| {
                let parts: Vec<&str> = [
                    tg_user.first_name.as_deref(),
                    tg_user.last_name.as_deref(),
                ]
                .iter()
                .filter_map(|p| *p)
                .collect();
                if parts.is_empty() {
                    format!("tg-player-{}", tg_user.id)
                } else {
                    parts.join(" ")
                }
            })
        });

        let (player, is_new) = self
            .player_repo
            .find_or_create(&wallet, &display_name, None)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, wallet = %wallet, "DB error during Telegram Mini App login");
                AppError::Internal(e)
            })?;

        if is_new {
            tracing::info!(wallet = %wallet, name = %display_name, "New Telegram Mini App player registered");
            if let Err(e) = self.agent_repo.create_agent_for_new_user(&wallet).await {
                tracing::error!(error = %e, wallet = %wallet, "Failed to generate AI agent for new Telegram player");
            }
        }

        let _ = ip_address;

        let token = AuthService::sign_token(&player).map_err(|e| {
            tracing::error!(error = %e, "Failed to sign JWT for Telegram Mini App player");
            AppError::Internal(e)
        })?;

        Ok(LoginResponse {
            token,
            player: PlayerInfo {
                id: player.id.map(|oid| oid.to_hex()).unwrap_or_default(),
                wallet_address: player.wallet_address,
                name: player.name,
            },
        })
    }

    /// Get a player's full profile with aggregated stats.
    pub async fn get_profile(
        &self,
        wallet_address: &str,
    ) -> Result<PlayerProfileResponse, AppError> {
        let wallet = wallet_address.trim().to_string();
        tracing::debug!(wallet = %wallet, "Fetching player profile");

        let player = self
            .player_repo
            .find_by_wallet(&wallet)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, wallet = %wallet, "DB error fetching player");
                AppError::Internal(e)
            })?
            .ok_or_else(|| {
                tracing::warn!(wallet = %wallet, "Player not found");
                AppError::NotFound("Player not found".to_string())
            })?;

        let global_entry = self
            .global_lb_repo
            .get_player_entry(&wallet)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch global leaderboard entry");
                AppError::Internal(e.to_string())
            })?;

        let (rank, total_score, level) = match global_entry {
            Some(entry) => {
                tracing::debug!(
                    rank = entry.rank,
                    level = entry.level,
                    "Player leaderboard entry found"
                );
                (Some(entry.rank), entry.score, entry.level)
            }
            None => {
                tracing::debug!(wallet = %wallet, "Player not ranked yet");
                (None, 0.0, 1)
            }
        };

        let game_scores = self
            .game_lb_service
            .fetch_scores_for_player(&wallet)
            .await
            .unwrap_or_default();

        tracing::debug!(games_played = game_scores.len(), "Game scores fetched");

        let game_scores_list: Vec<GameScoreEntry> = game_scores
            .into_iter()
            .map(|(id, score, weight, weighted, game_rank)| GameScoreEntry {
                identification: id,
                score,
                weight,
                weighted_score: weighted,
                rank: game_rank,
            })
            .collect();

        let kult_points = player
            .kult_points
            .or(player.lifetime_kult_points)
            .unwrap_or(0);

        let profile = PlayerProfile {
            wallet_address: wallet,
            username: player.name,
            rank,
            kult_points,
            kult_points_rank: None,
            total_score,
            level,
            total_games_played: game_scores_list.len() as u32,
            completed_quests: 0,
            game_scores_list,
            purchased_assets: extract_purchased_assets(player.metadata.as_ref()),
        };

        Ok(PlayerProfileResponse {
            cached: false,
            profile,
        })
    }

    /// Update a player's display name.
    pub async fn update_name(
        &self,
        wallet_address: &str,
        request: UpdateNameRequest,
    ) -> Result<UpdateNameResponse, AppError> {
        let new_name = request.name.trim();
        tracing::debug!(wallet = %wallet_address, new_name = %new_name, "Updating player name");

        if new_name.is_empty() {
            tracing::warn!(wallet = %wallet_address, "Attempted to set empty name");
            return Err(AppError::BadRequest("Name cannot be empty".to_string()));
        }

        if new_name.len() > 100 {
            tracing::warn!(wallet = %wallet_address, len = new_name.len(), "Name too long");
            return Err(AppError::BadRequest(
                "Name cannot exceed 100 characters".to_string(),
            ));
        }

        let updated = self
            .player_repo
            .update_name(wallet_address, new_name)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "DB error updating player name");
                AppError::Internal(e)
            })?
            .ok_or_else(|| {
                tracing::warn!(wallet = %wallet_address, "Player not found for name update");
                AppError::NotFound("Player not found".to_string())
            })?;

        tracing::info!(wallet = %wallet_address, new_name = %updated.name, "Player name updated");
        Ok(UpdateNameResponse { name: updated.name })
    }
}

fn extract_purchased_assets(metadata: Option<&Document>) -> Option<Value> {
    let game_assets = metadata.and_then(|m| m.get("gameAssets"))?;
    serde_json::to_value(game_assets).ok()
}
