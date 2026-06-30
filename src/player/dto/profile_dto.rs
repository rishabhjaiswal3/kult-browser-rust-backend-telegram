// src/player/model/player_profile.rs

use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

/// GET /api/player/profile - Full response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlayerProfileResponse {
    pub cached: bool,
    pub profile: PlayerProfile,
}

/// Aggregated player statistics
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlayerProfile {
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,

    /// Display name
    pub username: String,

    /// Global leaderboard rank (null if unranked)
    pub rank: Option<u32>,

    /// KULT Points balance from store_players.kultPoints.
    #[serde(rename = "kultPoints")]
    pub kult_points: i64,

    /// KULT Points rank. Not computed yet.
    #[serde(rename = "kultPointsRank", skip_serializing_if = "Option::is_none")]
    pub kult_points_rank: Option<u32>,

    /// Aggregated weighted score across all games
    #[serde(rename = "totalScore")]
    pub total_score: f64,

    /// Computed level (1-100, XP curve)
    pub level: u32,

    /// Number of games with recorded scores
    #[serde(rename = "totalGamesPlayed")]
    pub total_games_played: u32,

    /// Placeholder for future quest system
    #[serde(rename = "completedQuests")]
    pub completed_quests: u32,

    /// Per-game breakdown
    #[serde(rename = "gameScoresList")]
    pub game_scores_list: Vec<GameScoreEntry>,

    /// Marketplace-owned assets grouped by game (from player metadata.gameAssets).
    #[serde(rename = "purchasedAssets", skip_serializing_if = "Option::is_none")]
    pub purchased_assets: Option<Value>,
}

/// Individual game score breakdown
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GameScoreEntry {
    /// Game identifier (e.g., "zerogpool")
    pub identification: String,

    /// Raw score in this game
    pub score: f64,

    /// Weight multiplier from config
    pub weight: f64,

    /// score * weight
    #[serde(rename = "weightedScore")]
    pub weighted_score: f64,

    /// Rank in this specific game (null if unavailable)
    pub rank: Option<u32>,
}
