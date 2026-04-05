use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    NotificationCreated,
    NoteMentioned,
    NoteLiked,
    NoteCommented,
    UserFollowed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: EventType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl Event {
    pub fn new(event_type: EventType, payload: serde_json::Value) -> Self {
        Self {
            id: String::new(),
            event_type,
            payload,
            timestamp: Utc::now(),
        }
    }
}

// ── Typed payloads ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMentionedPayload {
    pub note_id: Uuid,
    pub note_title: String,
    pub mentioned_user_id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteLikedPayload {
    pub note_id: Uuid,
    pub note_title: String,
    pub note_author_id: Uuid,
    pub liked_by_id: Uuid,
    pub liked_by_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteCommentedPayload {
    pub note_id: Uuid,
    pub note_title: String,
    pub note_author_id: Uuid,
    pub comment_id: Uuid,
    pub commenter_id: Uuid,
    pub commenter_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFollowedPayload {
    pub follower_id: Uuid,
    pub follower_name: String,
    pub followed_id: Uuid,
}
