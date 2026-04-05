use sqlx::PgPool;
use uuid::Uuid;

use crate::notification::model::NotificationWithUnread;

use super::model::Notification;

pub struct NotificationRepository;

impl NotificationRepository {
    pub async fn insert(
        pool: &PgPool,
        user_id: Uuid,
        r#type: &str,
        title: &str,
        message: &str,
        data: Option<serde_json::Value>,
    ) -> Result<Notification, sqlx::Error> {
        sqlx::query_as!(
            Notification,
            r#"
            INSERT INTO notifications (user_id, type, title, message, data, read)
            VALUES ($1, $2, $3, $4, $5, false)
            RETURNING id, user_id, type, title, message, data, read, created_at
            "#,
            user_id,
            r#type,
            title,
            message,
            data,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_user(
        pool: &PgPool,
        user_id: Uuid,
        r#type: Option<&str>,
        read: Option<bool>,
        limit: i64,
    ) -> Result<Vec<NotificationWithUnread>, sqlx::Error> {
        sqlx::query_as!(
            NotificationWithUnread,
            r#"
        SELECT
            id, user_id, type, title, message, data, read, created_at,
            COUNT(*) FILTER (WHERE read = false) OVER () AS "unread_count!: i64"
        FROM notifications
        WHERE user_id = $1
        AND ($2::text IS NULL OR type = $2)
        AND ($3::bool IS NULL OR read = $3)
        ORDER BY created_at DESC
        LIMIT $4
        "#,
            user_id,
            r#type,
            read,
            limit,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Notification>, sqlx::Error> {
        sqlx::query_as!(
            Notification,
            r#"
            SELECT id, user_id, type, title, message, data, read, created_at
            FROM notifications
            WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id,
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn mark_read(
        pool: &PgPool,
        ids: &[Uuid],
        user_id: Uuid,
        read: bool,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            UPDATE notifications SET read = $1
            WHERE id = ANY($2) AND user_id = $3
            "#,
            read,
            ids,
            user_id,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"DELETE FROM notifications WHERE id = $1 AND user_id = $2"#,
            id,
            user_id,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn unread_count(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!: i64" FROM notifications WHERE user_id = $1 AND read = false"#,
            user_id,
        )
        .fetch_one(pool)
        .await
    }
}
