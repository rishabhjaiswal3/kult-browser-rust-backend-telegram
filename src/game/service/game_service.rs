use crate::game::dto::{
    AllGamesResponse, CategoriesResponse, GameDetailDto, GameDetailResponse, GameListItemDto,
};
use crate::game::model::{util, GameModel, Localized};
use crate::game::repository::GameModelRepository;
use crate::handler::AppError;
use serde_json::Value;

/// Service layer for Game operations.
#[derive(Clone)]
pub struct GameService {
    repo: GameModelRepository,
}

impl GameService {
    pub fn new(repo: GameModelRepository) -> Self {
        Self { repo }
    }

    /// Get all games with optional search and pagination.
    pub async fn get_all_games(
        &self,
        search: Option<String>,
        page: u32,
        page_size: u32,
    ) -> Result<AllGamesResponse, AppError> {
        tracing::debug!(search = ?search, page, page_size, "Fetching all games");

        let skip = ((page.saturating_sub(1)) * page_size) as i64;
        let limit = page_size as i64;

        let (games, total_count) = self
            .repo
            .find_all_paginated(search.as_deref(), skip, limit)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch games from DB");
                AppError::Internal(format!("Failed to fetch games: {}", e))
            })?;

        tracing::debug!(
            fetched = games.len(),
            total = total_count,
            "Games query completed"
        );

        let game_dtos: Vec<GameListItemDto> = games.into_iter().map(Self::to_list_item).collect();

        let total_pages = if total_count == 0 {
            0
        } else {
            ((total_count as f64) / (page_size as f64)).ceil() as u32
        };

        Ok(AllGamesResponse {
            games: game_dtos,
            total_count,
            page,
            page_size,
            total_pages,
        })
    }

    /// Get a single game by identification.
    pub async fn get_game_by_identification(
        &self,
        identification: &str,
    ) -> Result<GameDetailResponse, AppError> {
        tracing::debug!(identification = %identification, "Fetching game by ID");

        let game = self
            .repo
            .find_by_identification(identification)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, identification = %identification, "DB error fetching game");
                AppError::Internal(format!("Failed to fetch game: {}", e))
            })?
            .ok_or_else(|| {
                tracing::warn!(identification = %identification, "Game not found");
                AppError::NotFound(format!("Game '{}' not found", identification))
            })?;

        tracing::debug!(identification = %identification, "Game found");
        Ok(GameDetailResponse {
            game: Self::to_detail(game),
        })
    }

    /// Get all unique categories.
    pub async fn get_all_categories(&self) -> Result<CategoriesResponse, AppError> {
        tracing::debug!("Fetching all categories");

        let categories = self.repo.get_distinct_categories().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch categories from DB");
            AppError::Internal(format!("Failed to fetch categories: {}", e))
        })?;

        tracing::debug!(count = categories.len(), "Categories fetched");
        Ok(CategoriesResponse { categories })
    }

    fn to_list_item(game: GameModel) -> GameListItemDto {
        GameListItemDto {
            identification: game.identification,
            name: game.name,
            category: game.category,
            slogan: game.slogan,
            rating: game.rating,
            thumbnail: game.images.hero,
            is_downloadable: game.is_downloadable,
        }
    }

    fn to_detail(game: GameModel) -> GameDetailDto {
        GameDetailDto {
            identification: game.identification,
            name: game.name,
            url: game.url,
            category: game.category,
            about: normalize_about(game.about),
            rating: game.rating,
            thumbnail: game.images.hero,
            is_downloadable: game.is_downloadable,
        }
    }
}

fn normalize_about(value: Option<Value>) -> Option<Localized<String>> {
    let value = value?;

    if value.is_null() {
        return None;
    }

    if let Ok(localized) = serde_json::from_value::<Localized<String>>(value.clone()) {
        return Some(localized);
    }

    if let Some(text) = extract_about_text(&value) {
        return Some(util::create_localized(text));
    }

    None
}

fn extract_about_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => normalized_text(text),
        Value::Array(items) => {
            let sections: Vec<String> = items.iter().filter_map(extract_about_section).collect();

            if sections.is_empty() {
                None
            } else {
                Some(sections.join("\n\n"))
            }
        }
        Value::Object(_) => extract_about_section(value),
        _ => None,
    }
}

fn extract_about_section(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => normalized_text(text),
        Value::Object(map) => {
            let title = map
                .get("title")
                .and_then(Value::as_str)
                .and_then(normalized_text);

            let body = map
                .get("content")
                .and_then(extract_content_text)
                .or_else(|| map.get("data").and_then(extract_content_text));

            match (title, body) {
                (Some(title), Some(body)) => Some(format!("{title}\n{body}")),
                (Some(title), None) => Some(title),
                (None, Some(body)) => Some(body),
                (None, None) => None,
            }
        }
        _ => None,
    }
}

fn extract_content_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => normalized_text(text),
        Value::Array(items) => {
            let lines: Vec<String> = items
                .iter()
                .filter_map(|item| item.as_str().and_then(normalized_text))
                .collect();

            if lines.is_empty() {
                None
            } else {
                Some(lines.join("\n"))
            }
        }
        Value::Object(map) => map.get("data").and_then(extract_content_text),
        _ => None,
    }
}

fn normalized_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_about, GameService};
    use crate::game::model::{util, GameImages, GameModel};
    use mongodb::bson::oid::ObjectId;
    use serde_json::json;

    #[test]
    fn normalizes_rich_text_about_array_into_localized_text() {
        let about = json!([
            {
                "index": 0,
                "title": "Introduction",
                "content": {
                    "type": "paragraph",
                    "data": "Zero G Pool is a fresh twist on the classic 8 Ball Pool experience."
                }
            },
            {
                "index": 1,
                "title": "Features",
                "content": {
                    "type": "list",
                    "data": ["Classic Pool Feel", "Skill-Based"]
                }
            }
        ]);

        let normalized = normalize_about(Some(about)).expect("about should normalize");

        assert_eq!(
            normalized,
            util::create_localized(
                "Introduction\nZero G Pool is a fresh twist on the classic 8 Ball Pool experience.\n\nFeatures\nClassic Pool Feel\nSkill-Based".to_string()
            )
        );
    }

    #[test]
    fn drops_image_only_about_payloads() {
        let about = json!({
            "horizontal": {
                "url": "https://example.com/about-desktop.png"
            },
            "vertical": {
                "url": "https://example.com/about-mobile.png"
            }
        });

        assert!(normalize_about(Some(about)).is_none());
    }

    #[test]
    fn maps_is_downloadable_in_game_list_item() {
        let game = GameModel {
            id: ObjectId::new(),
            identification: "test-game".to_string(),
            name: util::create_localized("Test Game".to_string()),
            platform: "web".to_string(),
            url: "https://example.com/game".to_string(),
            images: GameImages::default(),
            is_released: true,
            is_downloadable: true,
            slogan: None,
            about: None,
            category: None,
            tags: None,
            rating: None,
            rating_count: None,
            metadata: None,
            created_at: None,
            updated_at: None,
        };

        let dto = GameService::to_list_item(game);

        assert!(dto.is_downloadable);
        let json = serde_json::to_value(&dto).expect("dto should serialize");
        assert_eq!(
            json.get("isDownloadable").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert!(json.get("is_downloadable").is_none());
    }
}
