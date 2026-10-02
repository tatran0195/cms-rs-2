//! Comment entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::common::{Id, Timestamp};

/// Comment entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Comment {
    pub id: Id,
    pub page_id: Id,
    pub user_id: Option<Id>,
    pub reader_id: Option<Id>,
    pub parent_id: Option<Id>,
    pub content: String,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Comment response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CommentResponse {
    pub id: Id,
    pub page_id: Id,
    pub user_id: Option<Id>,
    pub reader_id: Option<Id>,
    pub parent_id: Option<Id>,
    pub content: String,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Comment> for CommentResponse {
    fn from(comment: Comment) -> Self {
        Self {
            id: comment.id,
            page_id: comment.page_id,
            user_id: comment.user_id,
            reader_id: comment.reader_id,
            parent_id: comment.parent_id,
            content: comment.content,
            resolved: comment.resolved,
            resolved_at: comment.resolved_at,
            resolved_by: comment.resolved_by,
            created_at: comment.created_at,
            updated_at: comment.updated_at,
        }
    }
}

/// Create comment request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct CreateCommentRequest {
    #[validate(length(min = 1, message = "Page ID is required"))]
    pub page_id: String,
    #[validate(length(
        min = 1,
        max = 5000,
        message = "Content must be between 1 and 5000 characters"
    ))]
    pub content: String,
    #[serde(default)]
    pub parent_id: Option<Id>,
}

/// Update comment request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateCommentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<bool>,
}

/// Resolve comment request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ResolveCommentRequest {
    #[serde(default)]
    pub resolved: bool,
}

/// List comments query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListCommentsQuery {
    #[serde(default)]
    pub page_id: Option<Id>,
    #[serde(default)]
    pub parent_id: Option<Id>,
    #[serde(default)]
    pub resolved: Option<bool>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Comment with replies
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CommentWithReplies {
    #[serde(flatten)]
    pub comment: CommentResponse,
    pub replies: Vec<CommentResponse>,
}
