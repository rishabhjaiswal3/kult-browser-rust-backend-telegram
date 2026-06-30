use crate::config::CONFIG;
use crate::player::model::PlayerModel;
use futures::stream::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime as BsonDateTime, Document};
use mongodb::{Collection, Database};

/// Repository for Player CRUD operations.
#[derive(Clone)]
pub struct PlayerRepository {
    collection: Collection<PlayerModel>,
}

impl PlayerRepository {
    /// Create a new repository instance
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection(&CONFIG.db.players_collection),
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // READ OPERATIONS
    // ─────────────────────────────────────────────────────────────────────

    /// Find a player by their wallet address (case-sensitive)
    ///
    /// # Arguments
    /// * `wallet_address` - The Ethereum wallet address (case is preserved)
    ///
    /// # Returns
    /// * `Ok(Some(PlayerModel))` if found
    /// * `Ok(None)` if not found
    pub async fn find_by_wallet(
        &self,
        wallet_address: &str,
    ) -> Result<Option<PlayerModel>, String> {
        let wallet = wallet_address.trim();

        self.collection
            .find_one(doc! { "walletAddress": wallet })
            .await
            .map_err(|e| format!("Failed to find player: {}", e))
    }

    /// Find a player by their MongoDB ObjectId
    ///
    /// # Arguments
    /// * id - The ObjectId as a hex string
    ///
    /// # Returns
    /// * `Ok(Some(PlayerModel))` if found
    /// * `Ok(None)` if not found or invalid ObjectId
    pub async fn find_by_id(&self, id: &str) -> Result<Option<PlayerModel>, String> {
        let oid = ObjectId::parse_str(id).map_err(|_| format!("Invalid ObjectId: {}", id))?;

        self.collection
            .find_one(doc! { "_id": oid })
            .await
            .map_err(|e| format!("Failed to find player by id: {}", e))
    }

    /// Check if a wallet address is already registered
    pub async fn exists_by_wallet(&self, wallet_address: &str) -> Result<bool, String> {
        let wallet = wallet_address.trim();

        let count = self
            .collection
            .count_documents(doc! { "walletAddress": wallet })
            .await
            .map_err(|e| format!("Failed to check existence: {}", e))?;

        Ok(count > 0)
    }

    // ─────────────────────────────────────────────────────────────────────
    // WRITE OPERATIONS
    // ─────────────────────────────────────────────────────────────────────

    /// Create a new player.
    ///
    /// # Arguments
    /// * `wallet_address` - Ethereum wallet address
    /// * `name` - Display name
    /// * `metadata` - Optional arbitrary metadata
    ///
    /// # Returns
    /// * The created PlayerModel with its new ObjectId
    pub async fn create(
        &self,
        wallet_address: &str,
        name: &str,
        metadata: Option<Document>,
    ) -> Result<PlayerModel, String> {
        let now = BsonDateTime::now();
        let wallet = wallet_address.trim().to_string();

        let player = PlayerModel {
            id: None, // MongoDB will generate
            wallet_address: wallet,
            name: name.trim().to_string(),
            kult_points: None,
            lifetime_kult_points: None,
            metadata,
            referral_code: None,
            created_at: Some(now),
            updated_at: Some(now),
        };

        let result = self
            .collection
            .insert_one(&player)
            .await
            .map_err(|e| format!("Failed to create player: {}", e))?;

        // Return with the inserted ID
        Ok(PlayerModel {
            id: result.inserted_id.as_object_id(),
            ..player
        })
    }

    /// Find or create a player atomically (upsert pattern for login).
    ///
    /// Uses `find_one_and_update` with `upsert: true` to avoid race conditions
    /// when two requests for the same new wallet arrive simultaneously.
    ///
    /// # Arguments
    /// * `wallet_address` - Ethereum wallet address
    /// * `name` - Display name (only used for creation)
    /// * `metadata` - Optional metadata (only used for creation)
    ///
    /// # Returns
    /// * `(PlayerModel, bool)` - The player and whether it was newly created
    pub async fn find_or_create(
        &self,
        wallet_address: &str,
        name: &str,
        metadata: Option<Document>,
    ) -> Result<(PlayerModel, bool), String> {
        let wallet = wallet_address.trim().to_string();
        let now = BsonDateTime::now();

        let mut set_on_insert = doc! {
            "walletAddress": &wallet,
            "name": name.trim(),
            "createdAt": now,
            "updatedAt": now,
        };

        if let Some(meta) = metadata {
            set_on_insert.insert("metadata", meta);
        }

        // Atomic upsert: only sets fields on insert, returns the doc after the operation
        let result = self
            .collection
            .find_one_and_update(
                doc! { "walletAddress": &wallet },
                doc! { "$setOnInsert": set_on_insert },
            )
            .upsert(true)
            .return_document(mongodb::options::ReturnDocument::After)
            .await
            .map_err(|e| format!("Failed to upsert player: {}", e))?
            .ok_or_else(|| "Upsert returned no document".to_string())?;

        // If createdAt equals updatedAt and matches `now`, it was just created.
        // More reliably: check if the returned doc's createdAt matches our `now`.
        let is_new = result.created_at.map(|ca| ca == now).unwrap_or(false);

        Ok((result, is_new))
    }

    /// Update a player's display name.
    ///
    /// # Arguments
    /// * `wallet_address` - The player's wallet address
    /// * `new_name` - The new display name
    ///
    /// # Returns
    /// * `Ok(Some(PlayerModel))` - Updated player if found
    /// * `Ok(None)` - If player not found
    pub async fn update_name(
        &self,
        wallet_address: &str,
        new_name: &str,
    ) -> Result<Option<PlayerModel>, String> {
        let wallet = wallet_address.trim();
        let trimmed_name = new_name.trim();

        // Validate name is not empty
        if trimmed_name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }

        let result = self
            .collection
            .find_one_and_update(
                doc! { "walletAddress": wallet },
                doc! {
                    "$set": {
                        "name": trimmed_name,
                        "updatedAt": BsonDateTime::now(),
                    }
                },
            )
            .return_document(mongodb::options::ReturnDocument::After)
            .await
            .map_err(|e| format!("Failed to update name: {}", e))?;

        Ok(result)
    }

    /// Update a player's metadata.
    ///
    /// # Arguments
    /// * `wallet_address` - The player's wallet address
    /// * `metadata` - The new metadata document (replaces existing)
    pub async fn update_metadata(
        &self,
        wallet_address: &str,
        metadata: Document,
    ) -> Result<Option<PlayerModel>, String> {
        let wallet = wallet_address.trim();

        let result = self
            .collection
            .find_one_and_update(
                doc! { "walletAddress": wallet },
                doc! {
                    "$set": {
                        "metadata": metadata,
                        "updatedAt": BsonDateTime::now(),
                    }
                },
            )
            .return_document(mongodb::options::ReturnDocument::After)
            .await
            .map_err(|e| format!("Failed to update metadata: {}", e))?;

        Ok(result)
    }

    /// Add a purchased marketplace asset into player's per-game metadata bucket.
    pub async fn add_purchased_asset(
        &self,
        player_id: &str,
        game_identification: &str,
        item_id: &str,
        category: &str,
        item_name: &str,
        order_id: &str,
        tx_hash: Option<&str>,
        quantity: u32,
    ) -> Result<(), String> {
        let oid = ObjectId::parse_str(player_id)
            .map_err(|_| format!("Invalid ObjectId: {}", player_id))?;
        let game_key = game_identification.trim();
        if game_key.is_empty() {
            return Err("game_identification cannot be empty".to_string());
        }

        let owned_key = format!("metadata.gameAssets.{}.ownedItemIds", game_key);
        let purchases_key = format!("metadata.gameAssets.{}.purchases", game_key);

        let mut purchase_doc = doc! {
            "itemId": item_id,
            "category": category,
            "name": item_name,
            "quantity": i32::try_from(quantity).unwrap_or(1),
            "orderId": order_id,
            "purchasedAt": BsonDateTime::now(),
        };
        if let Some(hash) = tx_hash {
            if !hash.trim().is_empty() {
                purchase_doc.insert("txHash", hash);
            }
        }

        self.collection
            .update_one(
                doc! { "_id": oid },
                doc! {
                    "$set": { "updatedAt": BsonDateTime::now() },
                    "$addToSet": {
                        owned_key: item_id,
                        purchases_key: purchase_doc,
                    }
                },
            )
            .await
            .map_err(|e| format!("Failed to add purchased asset: {}", e))?;

        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────
    // BULK / QUERY OPERATIONS (for admin/analytics)
    // ─────────────────────────────────────────────────────────────────────

    /// Get total player count
    pub async fn count_all(&self) -> Result<u64, String> {
        self.collection
            .count_documents(doc! {})
            .await
            .map_err(|e| format!("Failed to count players: {}", e))
    }

    /// Find multiple players by wallet addresses
    ///
    /// # Arguments
    /// * `wallet_addresses` - List of wallet addresses to find
    ///
    /// # Returns
    /// * Vector of found players (may be fewer than input if some not found)
    pub async fn find_many_by_wallets(
        &self,
        wallet_addresses: &[String],
    ) -> Result<Vec<PlayerModel>, String> {
        let wallets: Vec<String> = wallet_addresses
            .iter()
            .map(|w| w.trim().to_string())
            .collect();

        let cursor = self
            .collection
            .find(doc! { "walletAddress": { "$in": &wallets } })
            .await
            .map_err(|e| format!("Failed to find players: {}", e))?;

        cursor
            .try_collect()
            .await
            .map_err(|e| format!("Failed to collect players: {}", e))
    }

    // ─────────────────────────────────────────────────────────────────────
    // REFERRAL OPERATIONS
    // ─────────────────────────────────────────────────────────────────────

    /// Find a player by their referral code
    pub async fn find_by_referral_code(&self, code: &str) -> Result<Option<PlayerModel>, String> {
        self.collection
            .find_one(doc! { "referralCode": code })
            .await
            .map_err(|e| format!("Failed to find player by referral code: {}", e))
    }

    /// Set a player's referral code
    pub async fn set_referral_code(
        &self,
        wallet_address: &str,
        code: &str,
    ) -> Result<Option<PlayerModel>, String> {
        let wallet = wallet_address.trim();

        let result = self
            .collection
            .find_one_and_update(
                doc! { "walletAddress": wallet },
                doc! { "$set": { "referralCode": code } },
            )
            .return_document(mongodb::options::ReturnDocument::After)
            .await
            .map_err(|e| format!("Failed to update referral code: {}", e))?;

        Ok(result)
    }
}
