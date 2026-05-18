use chrono::Utc;
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::FindOneAndUpdateOptions,
    Collection, Database,
};
use std::collections::HashSet;

use crate::config::CONFIG;
use crate::GameModel;

#[derive(Clone)]
pub struct GameModelRepository {
    collection: Collection<GameModel>,
}

impl GameModelRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<GameModel>(&CONFIG.db.games_collection),
        }
    }

    // Find a GameModel by its unique slug
    pub async fn find_by_identification(
        &self,
        identification: &str,
    ) -> Result<Option<GameModel>, mongodb::error::Error> {
        let mut filter = Self::released_filter();
        filter.insert("identification", identification);

        self.collection.find_one(filter).await
    }
    // Check if a GameModel exists
    pub async fn exists(&self, identification: &str) -> Result<bool, mongodb::error::Error> {
        let count = self
            .collection
            .count_documents(doc! { "identification": identification })
            .await?;
        Ok(count > 0)
    }

    pub async fn find_released_identifications(
        &self,
        identifications: &[String],
    ) -> Result<HashSet<String>, mongodb::error::Error> {
        if identifications.is_empty() {
            return Ok(HashSet::new());
        }

        let values = self
            .collection
            .distinct(
                "identification",
                doc! {
                    "identification": { "$in": identifications },
                    "$or": [
                        { "isReleased": true },
                        { "is_released": true }
                    ]
                },
            )
            .await?;

        Ok(values
            .into_iter()
            .filter_map(|value| value.as_str().map(|value| value.to_string()))
            .collect())
    }

    // Create a new GameModel
    pub async fn create(&self, game_model: &GameModel) -> Result<String, mongodb::error::Error> {
        let result = self.collection.insert_one(game_model).await?;
        Ok(result.inserted_id.as_object_id().unwrap().to_hex())
    }
    // Patch (partial update) - only updates provided fields
    pub async fn patch(
        &self,
        identification: &str,
        updates: mongodb::bson::Document,
    ) -> Result<Option<GameModel>, mongodb::error::Error> {
        let mut update_doc = doc! {
            "$set": {
                "updated_at": Utc::now()
            }
        };
        // Merge the provided updates into $set
        if let Some(set_doc) = updates.get("$set") {
            if let Some(set_obj) = set_doc.as_document() {
                for (key, value) in set_obj {
                    update_doc
                        .get_document_mut("$set")
                        .unwrap()
                        .insert(key.clone(), value.clone());
                }
            }
        }
        // Handle $unset if present
        if let Some(unset_doc) = updates.get("$unset") {
            update_doc.insert("$unset", unset_doc.clone());
        }
        let options = FindOneAndUpdateOptions::builder()
            .return_document(mongodb::options::ReturnDocument::After)
            .build();
        self.collection
            .find_one_and_update(doc! { "identification": identification }, update_doc)
            .with_options(options)
            .await
    }
    // Replace (full update) - replaces entire document
    pub async fn replace(
        &self,
        identification: &str,
        game_model: &GameModel,
    ) -> Result<Option<GameModel>, mongodb::error::Error> {
        let options = mongodb::options::FindOneAndReplaceOptions::builder()
            .return_document(mongodb::options::ReturnDocument::After)
            .upsert(true)
            .build();
        self.collection
            .find_one_and_replace(doc! { "identification": identification }, game_model)
            .with_options(options)
            .await
    }
    // Delete a GameModel
    pub async fn find_by_ids(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<GameModel>, mongodb::error::Error> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        // Also support fetching by 'identification' slug if IDs fail?
        // Actually, StoreLayout uses `identification` (slugs) as IDs usually.
        // Let's assume input is SLUGS (identification) based on my previous plan.

        let filter = doc! {
            "identification": { "$in": ids }
        };

        let mut cursor = self.collection.find(filter).await?;
        let mut games = Vec::new();
        while let Some(game) = cursor.try_next().await? {
            games.push(game);
        }
        Ok(games)
    }

    pub async fn delete(&self, identification: &str) -> Result<bool, mongodb::error::Error> {
        let result = self
            .collection
            .delete_one(doc! { "identification": identification })
            .await?;
        Ok(result.deleted_count > 0)
    }
    // Find all GameModels (with optional limit)
    pub async fn find_all(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<GameModel>, mongodb::error::Error> {
        let mut cursor = self.collection.find(doc! {}).await?;
        let mut game_models = Vec::new();
        let max = limit.unwrap_or(100) as usize;
        while let Some(game_model) = cursor.try_next().await? {
            game_models.push(game_model);
            if game_models.len() >= max {
                break;
            }
        }
        Ok(game_models)
    }

    /// Find all games with optional text search and pagination.
    /// Returns (games, total_count) tuple.
    pub async fn find_all_paginated(
        &self,
        search: Option<&str>,
        skip: i64,
        limit: i64,
    ) -> Result<(Vec<GameModel>, u64), mongodb::error::Error> {
        use mongodb::options::FindOptions;

        // Build filter based on search
        let search_filter = match search {
            Some(query) if !query.trim().is_empty() => {
                // Escape regex special chars manually to avoid regex crate dependency
                let escaped: String = query
                    .trim()
                    .chars()
                    .flat_map(|c| {
                        if "\\^$.|?*+()[]{}".contains(c) {
                            vec!['\\', c]
                        } else {
                            vec![c]
                        }
                    })
                    .collect();
                let pattern = format!("(?i){}", escaped);
                doc! {
                    "$or": [
                        { "identification": { "$regex": &pattern } },
                        { "name": { "$regex": &pattern } },
                        { "name.en": { "$regex": &pattern } },
                        { "name.default": { "$regex": &pattern } },
                        { "category": { "$regex": &pattern } },
                        { "tags": { "$regex": &pattern } }
                    ]
                }
            }
            _ => Document::new(),
        };

        let released_filter = Self::released_filter();
        let filter = if search_filter.is_empty() {
            released_filter
        } else {
            doc! {
                "$and": [
                    released_filter,
                    search_filter
                ]
            }
        };

        // Get total count first
        let total_count = self.collection.count_documents(filter.clone()).await?;

        // Fetch paginated results
        let options = FindOptions::builder()
            .skip(skip as u64)
            .limit(limit)
            .sort(doc! { "identification": 1 })
            .build();

        let mut cursor = self.collection.find(filter).with_options(options).await?;
        let mut games = Vec::new();
        while let Some(game) = cursor.try_next().await? {
            games.push(game);
        }

        Ok((games, total_count))
    }

    /// Get all distinct categories from games.
    pub async fn get_distinct_categories(&self) -> Result<Vec<String>, mongodb::error::Error> {
        let categories: Vec<String> = self
            .collection
            .distinct(
                "category",
                doc! {
                    "category": { "$ne": null },
                    "$or": [
                        { "isReleased": true },
                        { "is_released": true }
                    ]
                },
            )
            .await?
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        Ok(categories)
    }

    fn released_filter() -> mongodb::bson::Document {
        doc! {
            "$or": [
                { "isReleased": true },
                { "is_released": true }
            ]
        }
    }
}
