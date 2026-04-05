use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub r#type: String,
    pub title: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct NotificationWithUnread {
    pub id: Uuid,
    pub user_id: Uuid,
    pub r#type: String,
    pub title: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
    pub unread_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct NotificationQuery {
    pub r#type: Option<String>,
    pub read: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct MarkReadRequest {
    pub notification_ids: Vec<Uuid>,
    pub read: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub notifications: Vec<Notification>,
    pub unread_count: i64,
}
