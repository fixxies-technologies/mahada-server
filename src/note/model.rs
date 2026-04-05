use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Type, Clone)]
#[sqlx(type_name = "note_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NoteType {
    Research,
    Personal,
    Draft,
    Published,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub note_type: NoteType,
    pub research_fields: Vec<String>,
    pub tags: Vec<String>,
    pub is_published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub user_id: Uuid,
    pub community_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct NoteWithCountsAndTotal {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub note_type: NoteType,
    pub research_fields: Vec<String>,
    pub tags: Vec<String>,
    pub is_published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub user_id: Uuid,
    pub community_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author_full_name: String,
    pub author_username: String,
    pub author_profile_img: Option<String>,
    pub like_count: i64,
    pub comment_count: i64,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct NoteWithCounts {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub note_type: NoteType,
    pub research_fields: Vec<String>,
    pub tags: Vec<String>,
    pub is_published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub user_id: Uuid,
    pub community_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Joined author fields
    pub author_full_name: String,
    pub author_username: String,
    pub author_profile_img: Option<String>,
    // Counts
    pub like_count: i64,
    pub comment_count: i64,
}

// Request / Response types

#[derive(Debug, Deserialize)]
pub struct NoteQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub community_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
    pub content: String,
    pub note_type: Option<NoteType>,
    pub research_fields: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub community_id: Option<Uuid>,
    pub is_published: Option<bool>,
    pub tagged_users: Option<Vec<TaggedUser>>,
    pub image_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct TaggedUser {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct NoteListResponse {
    pub notes: Vec<NoteWithCounts>,
    pub total: i64,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub note_type: Option<NoteType>,
    pub research_fields: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub is_published: Option<bool>,
    pub community_id: Option<Uuid>,
    pub image_urls: Option<Vec<String>>,
    pub tagged_users: Option<Vec<TaggedUser>>,
}

#[derive(Debug, Deserialize)]
pub struct NoteByUserQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
