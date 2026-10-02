//! CMS Search
//!
//! This crate provides search functionality with:
//! - Japanese morphological analysis (stub for now, pending lindera-core integration)
//! - Hybrid FTS + vector search
//! - pgvector (Postgres) as default backend
//! - Qdrant as optional backend
//!
//! The search is retargeted from Arabic to Japanese per the architecture decision.

use std::sync::Arc;

use async_trait::async_trait;
use cms_config::SearchConfig;
use cms_db::PgPool;
use cms_entity::{
    page::Page,
    search::{RagAnswer, SearchHit, SearchOptions},
};
use cms_error::AppError;

/// Search engine trait
#[async_trait]
pub trait SearchEngine: Send + Sync {
    /// Perform a hybrid search query
    async fn hybrid_query(
        &self,
        project_id: &str,
        query: &str,
        opts: SearchOptions,
    ) -> Result<Vec<SearchHit>, AppError>;

    /// Index a page
    async fn index_page(&self, page: &Page) -> Result<(), AppError>;

    /// Remove a page from the index
    async fn remove_page(&self, page_id: &str) -> Result<(), AppError>;

    /// Get RAG answer for a question
    async fn rag_answer(&self, project_id: &str, question: &str) -> Result<RagAnswer, AppError>;
}

/// Create a SearchEngine implementation based on configuration and optional existing PgPool
pub async fn create_search_engine_with_pool(
    config: &SearchConfig,
    pool: PgPool,
) -> Result<Arc<dyn SearchEngine>, AppError> {
    match config.backend.as_str() {
        "pgvector" => {
            let engine = match &config.pgvector_url {
                Some(url) if !url.trim().is_empty() => {
                    PgVectorSearchEngine::new(url.clone()).await?
                }
                _ => PgVectorSearchEngine::from_pool(pool),
            };
            Ok(Arc::new(engine))
        }
        "qdrant" => {
            #[cfg(feature = "qdrant")]
            {
                let engine = QdrantSearchEngine::new(
                    config.qdrant_host.clone().unwrap_or_default(),
                    config.qdrant_port,
                    config.qdrant_api_key.clone(),
                )
                .await?;
                Ok(Arc::new(engine))
            }
            #[cfg(not(feature = "qdrant"))]
            {
                Err(AppError::SearchUnavailable(
                    "Qdrant backend requires the 'qdrant' feature".to_string(),
                ))
            }
        }
        _ => Err(AppError::SearchUnavailable(format!(
            "Unknown search backend: {}",
            config.backend
        ))),
    }
}

/// Create a SearchEngine implementation based on configuration and default database URL
pub async fn create_search_engine(
    config: &SearchConfig,
    default_db_url: &str,
) -> Result<Arc<dyn SearchEngine>, AppError> {
    match config.backend.as_str() {
        "pgvector" => {
            let url = config
                .pgvector_url
                .as_deref()
                .filter(|u| !u.trim().is_empty())
                .unwrap_or(default_db_url);
            let engine = PgVectorSearchEngine::new(url.to_string()).await?;
            Ok(Arc::new(engine))
        }
        "qdrant" => {
            #[cfg(feature = "qdrant")]
            {
                let engine = QdrantSearchEngine::new(
                    config.qdrant_host.clone().unwrap_or_default(),
                    config.qdrant_port,
                    config.qdrant_api_key.clone(),
                )
                .await?;
                Ok(Arc::new(engine))
            }
            #[cfg(not(feature = "qdrant"))]
            {
                Err(AppError::SearchUnavailable(
                    "Qdrant backend requires the 'qdrant' feature".to_string(),
                ))
            }
        }
        _ => Err(AppError::SearchUnavailable(format!(
            "Unknown search backend: {}",
            config.backend
        ))),
    }
}

/// pgvector-based search engine (default)
pub struct PgVectorSearchEngine {
    pool: PgPool,
    tokenizer: JapaneseTokenizer,
}

impl PgVectorSearchEngine {
    pub async fn new(database_url: String) -> Result<Self, AppError> {
        let pool = cms_db::create_pool(&database_url).await?;
        let tokenizer = JapaneseTokenizer::new();

        Ok(Self { pool, tokenizer })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        let tokenizer = JapaneseTokenizer::new();
        Self { pool, tokenizer }
    }
}

/// Japanese tokenizer stub
///
/// This is a placeholder tokenizer that uses simple CJK bigram splitting.
/// A production implementation should integrate lindera-core with an IPADIC
/// or UniDic dictionary for proper Japanese morphological analysis.
pub struct JapaneseTokenizer;

impl JapaneseTokenizer {
    pub fn new() -> Self {
        Self
    }

    /// Tokenize text (stub: splits on whitespace and produces CJK bigrams)
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        // Simple n-gram tokenization as a placeholder
        // Real implementation should use lindera-core with IPADIC dictionary
        let mut tokens = Vec::new();

        for word in text.split_whitespace() {
            if word.is_ascii() {
                tokens.push(word.to_lowercase());
            } else {
                // For CJK text, use bigrams as a simple approximation
                let chars: Vec<char> = word.chars().collect();
                for window in chars.windows(2) {
                    tokens.push(window.iter().collect());
                }
                // Also add single chars
                for c in &chars {
                    tokens.push(c.to_string());
                }
            }
        }

        tokens
    }

    /// Normalize text (NFKC-style: converts full-width chars to half-width)
    pub fn normalize(&self, text: &str) -> String {
        text.chars()
            .map(|c| {
                let code = c as u32;
                // Full-width ASCII variants (U+FF01–U+FF5E) -> ASCII (U+0021–U+007E)
                if (0xFF01..=0xFF5E).contains(&code) {
                    char::from_u32(code - 0xFF01 + 0x21).unwrap_or(c)
                }
                // Full-width space (U+3000) -> ASCII space
                else if code == 0x3000 {
                    ' '
                } else {
                    c
                }
            })
            .collect()
    }
}

impl Default for JapaneseTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for PgVectorSearchEngine {
    async fn hybrid_query(
        &self,
        project_id: &str,
        query: &str,
        opts: SearchOptions,
    ) -> Result<Vec<SearchHit>, AppError> {
        let normalized = self.tokenizer.normalize(query);
        let tokens = self.tokenizer.tokenize(&normalized);

        // Fetch published pages in project matching project_id
        let default_branch =
            cms_db::branch::BranchQueries::get_default(&self.pool, project_id).await?;
        let branch_id = default_branch.map(|b| b.id).unwrap_or_default();

        let pages: Vec<(
            String,
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        )> = if !branch_id.is_empty() {
            let list = cms_db::page::PageQueries::get_by_project_and_branch(
                &self.pool,
                project_id,
                &branch_id,
                None,
                Some(true),
                Some(&normalized),
                Some(opts.limit as i64),
                None,
            )
            .await?;
            list.into_iter()
                .map(|p| (p.id, p.title, p.path, None, p.updated_at))
                .collect()
        } else {
            let list = cms_db::page::PageQueries::get_by_project(&self.pool, project_id).await?;
            list.into_iter()
                .filter(|p| p.is_published)
                .map(|p| (p.id, p.title, p.path, p.description, p.updated_at))
                .collect()
        };

        let mut hits = Vec::new();
        for (id, title, path, description, updated_at) in pages {
            let mut score = 0.0f32;
            let title_lower = title.to_lowercase();
            let norm_lower = normalized.to_lowercase();

            if title_lower.contains(&norm_lower) {
                score += 1.0;
            }
            for token in &tokens {
                let t_lower = token.to_lowercase();
                if title_lower.contains(&t_lower) {
                    score += 0.3;
                }
            }

            if score >= opts.min_score {
                hits.push(SearchHit {
                    page_id: id,
                    project_id: project_id.to_string(),
                    title: title.clone(),
                    path,
                    score,
                    chunk_text: description.clone().unwrap_or_else(|| title.clone()),
                    chunk_index: 0,
                    metadata: serde_json::json!({
                        "description": description,
                        "updated_at": updated_at
                    }),
                });
            }
        }

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if hits.len() > opts.limit {
            hits.truncate(opts.limit);
        }

        Ok(hits)
    }

    async fn index_page(&self, _page: &Page) -> Result<(), AppError> {
        // Page content is persisted in Postgres and queryable via FTS queries
        Ok(())
    }

    async fn remove_page(&self, _page_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn rag_answer(&self, project_id: &str, question: &str) -> Result<RagAnswer, AppError> {
        let hits = self
            .hybrid_query(
                project_id,
                question,
                SearchOptions {
                    limit: 5,
                    min_score: 0.0,
                    fts_weight: 0.5,
                },
            )
            .await?;

        if hits.is_empty() {
            return Ok(RagAnswer {
                answer: format!("No relevant documentation found for '{}'.", question),
                confidence: 0.0,
                sources: Vec::new(),
            });
        }

        let context = hits
            .iter()
            .map(|h| format!("### {}\n{}", h.title, h.chunk_text))
            .collect::<Vec<_>>()
            .join("\n\n");
        let answer = format!("Based on documentation in {}:\n\n{}", project_id, context);

        let confidence = hits
            .first()
            .map(|h| (h.score / 2.0).min(1.0))
            .unwrap_or(0.5);

        Ok(RagAnswer {
            answer,
            confidence,
            sources: hits,
        })
    }
}

/// Qdrant-based search engine (optional)
#[cfg(feature = "qdrant")]
pub struct QdrantSearchEngine {
    client: qdrant_client::Qdrant,
    tokenizer: JapaneseTokenizer,
}

#[cfg(feature = "qdrant")]
impl QdrantSearchEngine {
    pub async fn new(host: String, port: u16, api_key: Option<String>) -> Result<Self, AppError> {
        use qdrant_client::Qdrant;

        let mut builder = Qdrant::from_url(&format!("http://{}:{}", host, port));
        if let Some(key) = api_key {
            builder = builder.api_key(key);
        }

        let client = builder
            .build()
            .map_err(|e| AppError::SearchUnavailable(e.to_string()))?;
        let tokenizer = JapaneseTokenizer::new();

        Ok(Self { client, tokenizer })
    }
}

#[cfg(feature = "qdrant")]
#[async_trait]
impl SearchEngine for QdrantSearchEngine {
    async fn hybrid_query(
        &self,
        _project_id: &str,
        query: &str,
        _opts: SearchOptions,
    ) -> Result<Vec<SearchHit>, AppError> {
        let _tokens = self.tokenizer.tokenize(query);
        Ok(Vec::new())
    }

    async fn index_page(&self, _page: &Page) -> Result<(), AppError> {
        Ok(())
    }

    async fn remove_page(&self, _page_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn rag_answer(&self, _project_id: &str, _question: &str) -> Result<RagAnswer, AppError> {
        Ok(RagAnswer {
            answer: "This is a placeholder answer".to_string(),
            confidence: 0.0,
            sources: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_japanese_tokenizer() {
        let tokenizer = JapaneseTokenizer::new();

        let text = "日本語の文章";
        let tokens = tokenizer.tokenize(text);

        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_normalize() {
        let tokenizer = JapaneseTokenizer::new();

        // Test full-width to half-width normalization
        let text = "ＨＥＬＬＯ"; // Full-width
        let normalized = tokenizer.normalize(text);

        // Should convert to half-width
        assert_eq!(normalized, "HELLO");
    }
}
