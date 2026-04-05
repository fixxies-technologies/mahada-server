use sqlx::PgPool;
use uuid::Uuid;

use crate::events::bus::EventBus;
use crate::events::model::{Event, EventType, NoteLikedPayload, NoteMentionedPayload};

use super::model::{CreateNoteRequest, Note, NoteListResponse, NoteWithCounts, UpdateNoteRequest};
use super::repository::NoteRepository;

#[derive(Debug)]
pub enum NoteError {
    NotFound,
    Unauthorized,
    NotCommunityMember,
    DatabaseError(sqlx::Error),
    EventError(anyhow::Error),
}

impl From<sqlx::Error> for NoteError {
    fn from(e: sqlx::Error) -> Self {
        NoteError::DatabaseError(e)
    }
}

impl From<anyhow::Error> for NoteError {
    fn from(e: anyhow::Error) -> Self {
        NoteError::EventError(e)
    }
}

pub struct NoteService;

impl NoteService {
    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
        offset: i64,
        search: Option<&str>,
        community_id: Option<Uuid>,
    ) -> Result<NoteListResponse, NoteError> {
        let rows =
            NoteRepository::find_many(pool, user_id, limit, offset, search, community_id).await?;
        let total = rows.first().map(|r| r.total).unwrap_or(0);
        let notes = rows.into_iter().map(Self::strip_total).collect();
        Ok(NoteListResponse {
            has_more: offset + limit < total,
            notes,
            total,
        })
    }

    pub async fn list_by_user(
        pool: &PgPool,
        owner_id: Uuid,
        requesting_user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<NoteListResponse, NoteError> {
        let rows =
            NoteRepository::find_by_user(pool, owner_id, requesting_user_id, limit, offset).await?;
        let total = rows.first().map(|r| r.total).unwrap_or(0);
        let notes = rows.into_iter().map(Self::strip_total).collect();
        Ok(NoteListResponse {
            has_more: offset + limit < total,
            notes,
            total,
        })
    }

    pub async fn get_by_id(pool: &PgPool, note_id: Uuid, user_id: Uuid) -> Result<Note, NoteError> {
        NoteRepository::find_by_id(pool, note_id, user_id)
            .await?
            .ok_or(NoteError::NotFound)
    }

    pub async fn create(
        pool: &PgPool,
        bus: &EventBus,
        user_id: Uuid,
        req: CreateNoteRequest,
        author_name: &str,
    ) -> Result<Note, NoteError> {
        if let Some(community_id) = req.community_id {
            if !NoteRepository::is_community_member(pool, community_id, user_id).await? {
                return Err(NoteError::NotCommunityMember);
            }
        }

        let is_published = req.is_published.unwrap_or(false);
        let note = NoteRepository::create(pool, user_id, &req, is_published).await?;

        if let Some(ref urls) = req.image_urls {
            if !urls.is_empty() {
                NoteRepository::create_images(pool, note.id, urls).await?;
            }
        }

        if let Some(ref tagged) = req.tagged_users {
            if !tagged.is_empty() {
                let user_ids: Vec<Uuid> = tagged.iter().map(|t| t.id).collect();
                NoteRepository::create_mentions(pool, note.id, &user_ids).await?;

                // Publish one event per mentioned user
                for mentioned_user_id in user_ids {
                    bus.publish(&Event::new(
                        EventType::NoteMentioned,
                        serde_json::to_value(NoteMentionedPayload {
                            note_id: note.id,
                            note_title: note.title.clone(),
                            mentioned_user_id,
                            author_id: user_id,
                            author_name: author_name.to_string(),
                        })
                        .unwrap_or_default(),
                    ))
                    .await?;
                }
            }
        }

        Ok(note)
    }

    pub async fn update(
        pool: &PgPool,
        bus: &EventBus,
        note_id: Uuid,
        user_id: Uuid,
        req: UpdateNoteRequest,
        author_name: &str,
    ) -> Result<Note, NoteError> {
        let note = NoteRepository::update(pool, note_id, user_id, &req)
            .await?
            .ok_or(NoteError::NotFound)?;

        if let Some(ref urls) = req.image_urls {
            NoteRepository::replace_images(pool, note_id, urls).await?;
        }

        if let Some(ref tagged) = req.tagged_users {
            let user_ids: Vec<Uuid> = tagged.iter().map(|t| t.id).collect();
            let newly_added = NoteRepository::sync_mentions(pool, note_id, &user_ids).await?;

            let title = req.title.as_deref().unwrap_or(&note.title);
            for mentioned_user_id in newly_added {
                bus.publish(&Event::new(
                    EventType::NoteMentioned,
                    serde_json::to_value(NoteMentionedPayload {
                        note_id,
                        note_title: title.to_string(),
                        mentioned_user_id,
                        author_id: user_id,
                        author_name: author_name.to_string(),
                    })
                    .unwrap_or_default(),
                ))
                .await?;
            }
        }

        Ok(note)
    }

    pub async fn delete(pool: &PgPool, note_id: Uuid, user_id: Uuid) -> Result<(), NoteError> {
        if !NoteRepository::delete(pool, note_id, user_id).await? {
            return Err(NoteError::NotFound);
        }
        Ok(())
    }

    pub async fn toggle_like(
        pool: &PgPool,
        bus: &EventBus,
        note_id: Uuid,
        user_id: Uuid,
        liker_name: &str,
    ) -> Result<bool, NoteError> {
        let liked = NoteRepository::toggle_like(pool, note_id, user_id).await?;

        if liked {
            if let Ok(Some(note)) = NoteRepository::find_by_id(pool, note_id, user_id).await {
                // Don't notify if you liked your own note
                if note.user_id != user_id {
                    bus.publish(&Event::new(
                        EventType::NoteLiked,
                        serde_json::to_value(NoteLikedPayload {
                            note_id,
                            note_title: note.title,
                            note_author_id: note.user_id,
                            liked_by_id: user_id,
                            liked_by_name: liker_name.to_string(),
                        })
                        .unwrap_or_default(),
                    ))
                    .await?;
                }
            }
        }

        Ok(liked)
    }

    fn strip_total(r: super::model::NoteWithCountsAndTotal) -> NoteWithCounts {
        NoteWithCounts {
            id: r.id,
            title: r.title,
            content: r.content,
            note_type: r.note_type,
            research_fields: r.research_fields,
            tags: r.tags,
            is_published: r.is_published,
            published_at: r.published_at,
            user_id: r.user_id,
            community_id: r.community_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
            author_full_name: r.author_full_name,
            author_username: r.author_username,
            author_profile_img: r.author_profile_img,
            like_count: r.like_count,
            comment_count: r.comment_count,
        }
    }
}
