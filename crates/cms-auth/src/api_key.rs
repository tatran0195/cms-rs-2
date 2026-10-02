//! API key authentication

use cms_db::auth::ApiKeyQueries;
use cms_entity::auth::{ApiKey, ApiKeyResponse};
use cms_error::AppError;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::AuthService;

/// Hash an API key for storage using cryptographic SHA-256
pub fn hash_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{}", hex::encode(result))
}

impl AuthService {
    /// Create a new API key for a user.
    ///
    /// Generates a random key, hashes it, and stores it.
    /// Returns `(ApiKey, raw_key)` — the raw key is only available once and should
    /// be shown to the user immediately.
    pub async fn create_api_key(
        &self,
        user_id: &str,
        name: &str,
    ) -> Result<(ApiKey, String), AppError> {
        // Generate a random key
        let raw_key = format!("nbl_{}", Uuid::new_v4().to_string().replace("-", ""));
        // Hash it for storage
        let hashed = hash_key(&raw_key);

        let key = ApiKeyQueries::create(&self.pool, user_id, name, &hashed).await?;
        Ok((key, raw_key))
    }

    /// Validate an API key (looks up by hashed value)
    pub async fn validate_api_key(&self, raw_key: &str) -> Result<Option<ApiKey>, AppError> {
        let hashed = hash_key(raw_key);
        ApiKeyQueries::get_by_key(&self.pool, &hashed).await
    }

    /// List API keys for a user
    pub async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyResponse>, AppError> {
        ApiKeyQueries::get_all_for_user(&self.pool, user_id).await
    }

    /// Delete an API key
    pub async fn delete_api_key(&self, key_id: &str) -> Result<bool, AppError> {
        ApiKeyQueries::delete(&self.pool, key_id).await
    }
}
