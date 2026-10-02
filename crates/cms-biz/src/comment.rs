//! Comment Business Logic
//!
//! This module contains business logic for comments on pages.

use cms_db::{comment::CommentQueries, page::PageQueries};
use cms_entity::{
    comment::{Comment, CommentResponse, CreateCommentRequest, UpdateCommentRequest},
    common::{Id, MemberRole, PaginatedResponse},
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Comment service
pub struct CommentService;

impl CommentService {
    /// Create a new comment on a page
    pub async fn create_comment(
        ctx: &BizContext,
        user_id: &str,
        page_id: &str,
        request: CreateCommentRequest,
    ) -> Result<CommentResponse, AppError> {
        // Verify page exists
        let page = PageQueries::get_by_id(&ctx.pool, page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        // Check if user has access to view the page
        ctx.authz
            .require_project_role(user_id, &page.project_id, MemberRole::Viewer)
            .await?;

        // Verify parent comment exists (if specified)
        if let Some(parent_id) = &request.parent_id {
            let _parent = CommentQueries::get_by_id(&ctx.pool, parent_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Parent comment not found".to_string()))?;
        }

        let comment = CommentQueries::create(
            &ctx.pool,
            page_id,
            Some(user_id),
            None, // reader_id
            request.parent_id.as_deref(),
            &request.content,
        )
        .await?;

        Ok(comment.into())
    }

    /// Get a comment by ID
    pub async fn get_comment(
        ctx: &BizContext,
        user_id: &str,
        comment_id: &str,
    ) -> Result<CommentResponse, AppError> {
        let comment = CommentQueries::get_by_id(&ctx.pool, comment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

        // Check if user has access to the page
        let page = PageQueries::get_by_id(&ctx.pool, &comment.page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        ctx.authz
            .require_project_role(user_id, &page.project_id, MemberRole::Viewer)
            .await?;

        Ok(comment.into())
    }

    /// List comments for a page
    pub async fn list_comments(
        ctx: &BizContext,
        user_id: &str,
        page_id: &str,
        parent_id: Option<&str>,
        resolved: Option<bool>,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<CommentResponse>, AppError> {
        // Verify page exists
        let _page_entity = PageQueries::get_by_id(&ctx.pool, page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        // Check if user has access to the page
        ctx.authz
            .require_project_role(user_id, &_page_entity.project_id, MemberRole::Viewer)
            .await?;

        let comments = CommentQueries::get_by_page(
            &ctx.pool,
            page_id,
            parent_id,
            resolved,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = CommentQueries::count_by_page(&ctx.pool, page_id, parent_id, resolved).await?;

        Ok(PaginatedResponse::new(
            comments.into_iter().map(|c| c.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// Update a comment
    pub async fn update_comment(
        ctx: &BizContext,
        user_id: &str,
        comment_id: &str,
        request: UpdateCommentRequest,
    ) -> Result<CommentResponse, AppError> {
        let comment = CommentQueries::get_by_id(&ctx.pool, comment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

        // Check if user is the author or has admin rights
        if comment.user_id.as_deref() != Some(user_id) {
            // Check if user has admin role in the project
            let page = PageQueries::get_by_id(&ctx.pool, &comment.page_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

            ctx.authz
                .require_project_role(user_id, &page.project_id, MemberRole::Admin)
                .await?;
        }

        let updated = CommentQueries::update(
            &ctx.pool,
            comment_id,
            request.content.as_deref(),
            request.resolved,
        )
        .await?;

        Ok(updated.into())
    }

    /// Delete a comment
    pub async fn delete_comment(
        ctx: &BizContext,
        user_id: &str,
        comment_id: &str,
    ) -> Result<bool, AppError> {
        let comment = CommentQueries::get_by_id(&ctx.pool, comment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

        // Check if user is the author or has admin rights
        if comment.user_id.as_deref() != Some(user_id) {
            // Check if user has admin role in the project
            let page = PageQueries::get_by_id(&ctx.pool, &comment.page_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

            ctx.authz
                .require_project_role(user_id, &page.project_id, MemberRole::Admin)
                .await?;
        }

        CommentQueries::delete(&ctx.pool, comment_id).await
    }

    /// Resolve a comment (admin only)
    pub async fn resolve_comment(
        ctx: &BizContext,
        user_id: &str,
        comment_id: &str,
    ) -> Result<CommentResponse, AppError> {
        let comment = CommentQueries::get_by_id(&ctx.pool, comment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

        // Check if user has admin role in the project
        let page = PageQueries::get_by_id(&ctx.pool, &comment.page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        ctx.authz
            .require_project_role(user_id, &page.project_id, MemberRole::Admin)
            .await?;

        let updated = CommentQueries::update(&ctx.pool, comment_id, None, Some(true)).await?;

        Ok(updated.into())
    }

    /// Unresolve a comment (admin only)
    pub async fn unresolve_comment(
        ctx: &BizContext,
        user_id: &str,
        comment_id: &str,
    ) -> Result<CommentResponse, AppError> {
        let comment = CommentQueries::get_by_id(&ctx.pool, comment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

        // Check if user has admin role in the project
        let page = PageQueries::get_by_id(&ctx.pool, &comment.page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        ctx.authz
            .require_project_role(user_id, &page.project_id, MemberRole::Admin)
            .await?;

        let updated = CommentQueries::update(&ctx.pool, comment_id, None, Some(false)).await?;

        Ok(updated.into())
    }

    /// Get comment with replies
    pub async fn get_comment_with_replies(
        ctx: &BizContext,
        user_id: &str,
        comment_id: &str,
    ) -> Result<cms_entity::comment::CommentWithReplies, AppError> {
        let comment = Self::get_comment(ctx, user_id, comment_id).await?;
        let replies = CommentQueries::get_by_page(
            &ctx.pool,
            &comment.page_id,
            Some(comment_id),
            None,
            None,
            None,
        )
        .await?;
        Ok(cms_entity::comment::CommentWithReplies {
            comment,
            replies: replies.into_iter().map(|r| r.into()).collect(),
        })
    }
}
