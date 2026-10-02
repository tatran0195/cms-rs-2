//! Notification Business Logic
//!
//! This module contains business logic for notifications.

use chrono::Utc;
use cms_db::notification::NotificationQueries;
use cms_entity::{
    common::{Id, PaginatedResponse},
    notification::{Notification, NotificationResponse, NotificationStatus, NotificationType},
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Notification service
pub struct NotificationService;

impl NotificationService {
    /// Create a notification
    pub async fn create_notification(
        ctx: &BizContext,
        user_id: &str,
        notification_type: NotificationType,
        title: &str,
        message: &str,
        data: serde_json::Value,
    ) -> Result<NotificationResponse, AppError> {
        let notification = NotificationQueries::create(
            &ctx.pool,
            user_id,
            notification_type,
            title,
            message,
            data,
        )
        .await?;

        Ok(notification.into())
    }

    /// Get a notification
    pub async fn get_notification(
        ctx: &BizContext,
        user_id: &str,
        notification_id: &str,
    ) -> Result<NotificationResponse, AppError> {
        let notification = NotificationQueries::get_by_id(&ctx.pool, notification_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Notification not found".to_string()))?;

        // Verify notification belongs to user
        if notification.user_id != user_id {
            return Err(AppError::AccessDenied(
                "Notification does not belong to this user".to_string(),
            ));
        }

        Ok(notification.into())
    }

    /// List notifications for a user
    pub async fn list_notifications(
        ctx: &BizContext,
        user_id: &str,
        status: Option<NotificationStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<NotificationResponse>, AppError> {
        let notifications = NotificationQueries::get_by_user(
            &ctx.pool,
            user_id,
            status.as_ref(),
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = NotificationQueries::count_by_user(&ctx.pool, user_id, status.as_ref()).await?;

        Ok(PaginatedResponse::new(
            notifications.into_iter().map(|n| n.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// Mark notification as read
    pub async fn mark_as_read(
        ctx: &BizContext,
        user_id: &str,
        notification_id: &str,
    ) -> Result<NotificationResponse, AppError> {
        let notification = NotificationQueries::get_by_id(&ctx.pool, notification_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Notification not found".to_string()))?;

        // Verify notification belongs to user
        if notification.user_id != user_id {
            return Err(AppError::AccessDenied(
                "Notification does not belong to this user".to_string(),
            ));
        }

        let updated = NotificationQueries::update_status(
            &ctx.pool,
            notification_id,
            NotificationStatus::READ,
        )
        .await?;

        Ok(updated.into())
    }

    /// Mark all notifications as read
    pub async fn mark_all_as_read(ctx: &BizContext, user_id: &str) -> Result<u64, AppError> {
        NotificationQueries::mark_all_as_read(&ctx.pool, user_id).await
    }

    /// Delete a notification
    pub async fn delete_notification(
        ctx: &BizContext,
        user_id: &str,
        notification_id: &str,
    ) -> Result<bool, AppError> {
        let notification = NotificationQueries::get_by_id(&ctx.pool, notification_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Notification not found".to_string()))?;

        // Verify notification belongs to user
        if notification.user_id != user_id {
            return Err(AppError::AccessDenied(
                "Notification does not belong to this user".to_string(),
            ));
        }

        NotificationQueries::delete(&ctx.pool, notification_id).await
    }

    /// Get unread notification count
    pub async fn get_unread_count(ctx: &BizContext, user_id: &str) -> Result<u64, AppError> {
        let count = NotificationQueries::count_unread_by_user(&ctx.pool, user_id).await?;

        Ok(count as u64)
    }

    /// Mark multiple notifications as read
    pub async fn mark_notifications_read(
        ctx: &BizContext,
        user_id: &str,
        notification_ids: &[String],
    ) -> Result<(), AppError> {
        for id in notification_ids {
            if let Err(e) = Self::mark_as_read(ctx, user_id, id).await {
                tracing::warn!(
                    "Failed to mark notification {} as read for user {}: {}",
                    id,
                    user_id,
                    e
                );
            }
        }
        Ok(())
    }

    /// Mark all notifications as read
    pub async fn mark_all_notifications_read(
        ctx: &BizContext,
        user_id: &str,
    ) -> Result<(), AppError> {
        let _ = Self::mark_all_as_read(ctx, user_id).await?;
        Ok(())
    }

    /// Archive notification
    pub async fn archive_notification(
        ctx: &BizContext,
        user_id: &str,
        notification_id: &str,
    ) -> Result<(), AppError> {
        let _ = Self::delete_notification(ctx, user_id, notification_id).await?;
        Ok(())
    }

    /// Get notification count
    pub async fn get_notification_count(
        ctx: &BizContext,
        user_id: &str,
    ) -> Result<cms_entity::notification::NotificationCountResponse, AppError> {
        let total = NotificationQueries::count_by_user(&ctx.pool, user_id, None).await?;
        let unread = NotificationQueries::count_unread_by_user(&ctx.pool, user_id).await?;
        Ok(cms_entity::notification::NotificationCountResponse {
            total,
            unread,
            by_type: std::collections::HashMap::new(),
        })
    }
}
