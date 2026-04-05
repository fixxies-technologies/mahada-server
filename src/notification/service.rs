use sqlx::PgPool;
use uuid::Uuid;

use crate::events::model::{
    EventType, NoteCommentedPayload, NoteLikedPayload, NoteMentionedPayload, UserFollowedPayload,
};
use crate::sse::manager::SseManager;

use super::model::{Notification, NotificationListResponse};
use super::repository::NotificationRepository;
use std::sync::Arc;

#[derive(Debug)]
pub enum NotificationError {
    NotFound,
    DatabaseError(sqlx::Error),
}

impl From<sqlx::Error> for NotificationError {
    fn from(e: sqlx::Error) -> Self {
        NotificationError::DatabaseError(e)
    }
}

pub struct NotificationService;

impl NotificationService {
    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        r#type: Option<&str>,
        read: Option<bool>,
    ) -> Result<NotificationListResponse, NotificationError> {
        let rows = NotificationRepository::find_by_user(pool, user_id, r#type, read, 50).await?;
        let unread_count = rows.first().map(|r| r.unread_count).unwrap_or(0);

        let notifications = rows
            .into_iter()
            .map(|r| Notification {
                id: r.id,
                user_id: r.user_id,
                r#type: r.r#type,
                title: r.title,
                message: r.message,
                data: r.data,
                read: r.read,
                created_at: r.created_at,
            })
            .collect();

        Ok(NotificationListResponse {
            notifications,
            unread_count,
        })
    }

    pub async fn get_by_id(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Notification, NotificationError> {
        NotificationRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or(NotificationError::NotFound)
    }

    pub async fn mark_read(
        pool: &PgPool,
        ids: &[Uuid],
        user_id: Uuid,
        read: bool,
    ) -> Result<u64, NotificationError> {
        Ok(NotificationRepository::mark_read(pool, ids, user_id, read).await?)
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), NotificationError> {
        if !NotificationRepository::delete(pool, id, user_id).await? {
            return Err(NotificationError::NotFound);
        }
        Ok(())
    }

    pub async fn unread_count(pool: &PgPool, user_id: Uuid) -> Result<i64, NotificationError> {
        Ok(NotificationRepository::unread_count(pool, user_id).await?)
    }

    pub async fn handle_event(
        pool: &PgPool,
        sse: &Arc<SseManager>,
        event_type: &EventType,
        payload: &serde_json::Value,
    ) -> Result<(), anyhow::Error> {
        match event_type {
            EventType::NoteMentioned => {
                let p: NoteMentionedPayload = serde_json::from_value(payload.clone())?;
                let notification = NotificationRepository::insert(
                    pool,
                    p.mentioned_user_id,
                    "mention",
                    "You were mentioned in a note",
                    &format!("{} mentioned you in \"{}\"", p.author_name, p.note_title),
                    Some(serde_json::to_value(&p)?),
                )
                .await?;

                sse.broadcast_to_user(
                    p.mentioned_user_id,
                    "notification",
                    serde_json::to_value(&notification)?,
                )
                .await
                .ok(); // SSE push is best-effort
            }

            EventType::NoteLiked => {
                let p: NoteLikedPayload = serde_json::from_value(payload.clone())?;
                let notification = NotificationRepository::insert(
                    pool,
                    p.note_author_id,
                    "like",
                    "Someone liked your note",
                    &format!("{} liked \"{}\"", p.liked_by_name, p.note_title),
                    Some(serde_json::to_value(&p)?),
                )
                .await?;

                sse.broadcast_to_user(
                    p.note_author_id,
                    "notification",
                    serde_json::to_value(&notification)?,
                )
                .await
                .ok();
            }

            EventType::NoteCommented => {
                let p: NoteCommentedPayload = serde_json::from_value(payload.clone())?;
                let notification = NotificationRepository::insert(
                    pool,
                    p.note_author_id,
                    "comment",
                    "Someone commented on your note",
                    &format!("{} commented on \"{}\"", p.commenter_name, p.note_title),
                    Some(serde_json::to_value(&p)?),
                )
                .await?;

                sse.broadcast_to_user(
                    p.note_author_id,
                    "notification",
                    serde_json::to_value(&notification)?,
                )
                .await
                .ok();
            }

            EventType::UserFollowed => {
                let p: UserFollowedPayload = serde_json::from_value(payload.clone())?;
                let notification = NotificationRepository::insert(
                    pool,
                    p.followed_id,
                    "follow",
                    "Someone followed you",
                    &format!("{} started following you", p.follower_name),
                    Some(serde_json::to_value(&p)?),
                )
                .await?;

                sse.broadcast_to_user(
                    p.followed_id,
                    "notification",
                    serde_json::to_value(&notification)?,
                )
                .await
                .ok();
            }

            // Nothing to do for this type
            EventType::NotificationCreated => {}
        }

        Ok(())
    }
}
