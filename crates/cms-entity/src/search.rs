//! Search entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Search index run status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "\"SearchIndexRunStatus\"", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SearchIndexRunStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Search index run entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchIndexRun {
    pub id: Id,
    pub project_id: Id,
    pub branch_id: Option<Id>,
    pub language_id: Option<Id>,
    pub status: SearchIndexRunStatus,
    pub pages_indexed: i32,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Search index run response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchIndexRunResponse {
    pub id: Id,
    pub project_id: Id,
    pub branch_id: Option<Id>,
    pub language_id: Option<Id>,
    pub status: SearchIndexRunStatus,
    pub pages_indexed: i32,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SearchIndexRun> for SearchIndexRunResponse {
    fn from(run: SearchIndexRun) -> Self {
        Self {
            id: run.id,
            project_id: run.project_id,
            branch_id: run.branch_id,
            language_id: run.language_id,
            status: run.status,
            pages_indexed: run.pages_indexed,
            error_message: run.error_message,
            started_at: run.started_at,
            completed_at: run.completed_at,
            created_at: run.created_at,
            updated_at: run.updated_at,
        }
    }
}

/// Create search index run request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateSearchIndexRunRequest {
    pub project_id: Id,
    #[serde(default)]
    pub branch_id: Option<Id>,
    #[serde(default)]
    pub language_id: Option<Id>,
}

/// Page embedding entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PageEmbedding {
    pub id: Id,
    pub page_id: Id,
    pub project_id: Id,
    pub embedding: Vec<f32>,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Page embedding response (without the full embedding vector for API responses)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PageEmbeddingResponse {
    pub id: Id,
    pub page_id: Id,
    pub project_id: Id,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub metadata: serde_json::Value,
    pub embedding_dimension: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PageEmbedding> for PageEmbeddingResponse {
    fn from(embedding: PageEmbedding) -> Self {
        Self {
            id: embedding.id,
            page_id: embedding.page_id,
            project_id: embedding.project_id,
            chunk_text: embedding.chunk_text,
            chunk_index: embedding.chunk_index,
            metadata: embedding.metadata,
            embedding_dimension: embedding.embedding.len(),
            created_at: embedding.created_at,
            updated_at: embedding.updated_at,
        }
    }
}

/// Search request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SearchRequest {
    pub query: String,
    pub project_id: Id,
    #[serde(default)]
    pub branch_id: Option<Id>,
    #[serde(default)]
    pub language_id: Option<Id>,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: Option<i32>,
}

fn default_limit() -> i32 {
    10
}

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchResultItem {
    pub page_id: Id,
    pub project_id: Id,
    pub title: String,
    pub path: String,
    pub score: f32,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub metadata: serde_json::Value,
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResultItem>,
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
}

/// Reindex request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ReindexRequest {
    pub project_id: Id,
    #[serde(default)]
    pub branch_id: Option<Id>,
    #[serde(default)]
    pub language_id: Option<Id>,
    #[serde(default = "default_reindex_full")]
    pub full_reindex: bool,
}

fn default_reindex_full() -> bool {
    false
}

/// List search index runs query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListSearchIndexRunsQuery {
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub status: Option<SearchIndexRunStatus>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Search options for hybrid search
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SearchOptions {
    /// Maximum number of results to return
    #[serde(default = "default_search_limit")]
    pub limit: usize,
    /// Minimum relevance score (0.0 to 1.0)
    #[serde(default)]
    pub min_score: f32,
    /// Weight given to FTS results vs vector results (0.0 = all vector, 1.0 = all FTS)
    #[serde(default = "default_fts_weight")]
    pub fts_weight: f32,
}

fn default_search_limit() -> usize {
    10
}
fn default_fts_weight() -> f32 {
    0.5
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: default_search_limit(),
            min_score: 0.0,
            fts_weight: default_fts_weight(),
        }
    }
}

/// A single search result hit
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchHit {
    pub page_id: Id,
    pub project_id: Id,
    pub title: String,
    pub path: String,
    pub score: f32,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub metadata: serde_json::Value,
}

/// RAG (Retrieval Augmented Generation) answer
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RagAnswer {
    /// The generated answer
    pub answer: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Source pages used to generate the answer
    pub sources: Vec<SearchHit>,
}

/// Request to index a specific page
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct IndexPageRequest {
    pub page_id: Id,
    pub project_id: Id,
    #[serde(default)]
    pub branch_id: Option<Id>,
    #[serde(default)]
    pub language_id: Option<Id>,
}
