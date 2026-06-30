// src/player/model/player_model.rs

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

/// Core player identity stored in MongoDB.
/// Collection: `store_players`
///
/// Note: Timestamp fields use skip_deserializing to handle legacy data
/// that may have RFC 3339 strings instead of BSON DateTime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerModel {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Unique identifier - Ethereum wallet address (case-preserved, trimmed)
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,

    /// Display name (user-editable)
    pub name: String,

    /// KULT Points mirrored from Creator Studio / rewards systems.
    #[serde(rename = "kultPoints", default)]
    pub kult_points: Option<i64>,

    #[serde(rename = "lifetimeKultPoints", default)]
    pub lifetime_kult_points: Option<i64>,

    /// Arbitrary metadata (e.g., avatar URL, preferences)
    /// Using Option<Document> for flexibility - can store any BSON
    #[serde(default)]
    pub metadata: Option<mongodb::bson::Document>,

    /// Auto-generated referral code (e.g., "x7B9qz")
    #[serde(
        rename = "referralCode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub referral_code: Option<String>,

    /// Timestamps are skipped during deserialization to handle mixed formats
    #[serde(rename = "createdAt", default, skip_deserializing)]
    pub created_at: Option<mongodb::bson::DateTime>,

    #[serde(rename = "updatedAt", default, skip_deserializing)]
    pub updated_at: Option<mongodb::bson::DateTime>,
}
