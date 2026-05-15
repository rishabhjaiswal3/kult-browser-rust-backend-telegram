// src/game/dto/games_dto.rs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::game::model::{Localized, OrientedImage};

/// DTO for a game item in a list (minimal fields for cards/lists)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameListItemDto {
    pub identification: String,
    pub name: Localized<String>,
    pub thumbnail: OrientedImage,
    #[serde(rename = "isDownloadable", default)]
    pub is_downloadable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slogan: Option<Localized<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,
}

/// DTO for single game detail view (includes url and more fields)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameDetailDto {
    pub identification: String,
    pub name: Localized<String>,
    pub url: String,
    pub thumbnail: OrientedImage,
    #[serde(rename = "isDownloadable", default)]
    pub is_downloadable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<Localized<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,
}

/// Paginated response for all games list
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AllGamesResponse {
    pub games: Vec<GameListItemDto>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    pub page: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
}

/// Response for all categories
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CategoriesResponse {
    pub categories: Vec<String>,
}

/// Response wrapper for single game
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameDetailResponse {
    pub game: GameDetailDto,
}
