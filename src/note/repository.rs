use sqlx::PgPool;
use uuid::Uuid;

use super::model::{CreateNoteRequest, Note, NoteType, NoteWithCountsAndTotal, UpdateNoteRequest};

pub struct NoteRepository;

impl NoteRepository {
    pub async fn find_many(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
        offset: i64,
        search: Option<&str>,
        community_id: Option<Uuid>,
    ) -> Result<Vec<NoteWithCountsAndTotal>, sqlx::Error> {
        let search_pattern = search.map(|s| format!("%{}%", s));

        sqlx::query_as!(
            NoteWithCountsAndTotal,
            r#"
            WITH filtered AS (
                SELECT
                    n.id, n.title, n.content,
                    n.note_type,
                    n.research_fields, n.tags,
                    n.is_published, n.published_at,
                    n.user_id, n.community_id,
                    n.created_at, n.updated_at,
                    u.full_name  AS author_full_name,
                    u.username   AS author_username,
                    u.profile_img AS author_profile_img,
                    COUNT(DISTINCT l.id) AS like_count,
                    COUNT(DISTINCT c.id) AS comment_count
                FROM notes n
                JOIN users u ON u.id = n.user_id
                LEFT JOIN note_likes l    ON l.note_id = n.id
                LEFT JOIN note_comments c ON c.note_id = n.id
                WHERE (
                    n.user_id = $1
                    OR n.is_published = true
                    OR EXISTS (
                        SELECT 1 FROM community_memberships cm
                        WHERE cm.community_id = n.community_id
                        AND cm.user_id = $1
                    )
                )
                AND ($4::uuid IS NULL OR n.community_id = $4)
                AND ($5::text IS NULL OR (
                    n.title ILIKE $5 OR n.content ILIKE $5
                ))
                GROUP BY n.id, u.full_name, u.username, u.profile_img
                ORDER BY n.created_at DESC
            )
            SELECT
                *,
                COUNT(*) OVER () AS "total!: i64",
                note_type AS "note_type: NoteType",
                like_count AS "like_count!: i64",
                comment_count AS "comment_count!: i64"
            FROM filtered
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit,
            offset,
            community_id,
            search_pattern,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &PgPool,
        note_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Note>, sqlx::Error> {
        sqlx::query_as!(
            Note,
            r#"
            SELECT
                id, title, content,
                note_type AS "note_type: NoteType",
                research_fields, tags, is_published,
                published_at, user_id, community_id,
                created_at, updated_at
            FROM notes
            WHERE id = $1
            AND (
                user_id = $2
                OR is_published = true
                OR EXISTS (
                    SELECT 1 FROM community_memberships cm
                    WHERE cm.community_id = community_id
                    AND cm.user_id = $2
                )
            )
            "#,
            note_id,
            user_id,
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_user(
        pool: &PgPool,
        owner_id: Uuid,
        requesting_user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<NoteWithCountsAndTotal>, sqlx::Error> {
        sqlx::query_as!(
            NoteWithCountsAndTotal,
            r#"
            WITH filtered AS (
                SELECT
                    n.id, n.title, n.content,
                    n.note_type,
                    n.research_fields, n.tags,
                    n.is_published, n.published_at,
                    n.user_id, n.community_id,
                    n.created_at, n.updated_at,
                    u.full_name  AS author_full_name,
                    u.username   AS author_username,
                    u.profile_img AS author_profile_img,
                    COUNT(DISTINCT l.id) AS like_count,
                    COUNT(DISTINCT c.id) AS comment_count
                FROM notes n
                JOIN users u ON u.id = n.user_id
                LEFT JOIN note_likes l ON l.note_id = n.id
                LEFT JOIN note_comments c ON c.note_id = n.id
                WHERE n.user_id = $1
                AND (n.is_published = true OR n.user_id = $2)
                GROUP BY n.id, u.full_name, u.username, u.profile_img
                ORDER BY n.created_at DESC
            )
            SELECT
                *,
                COUNT(*) OVER () AS "total!: i64",
                note_type AS "note_type: NoteType",
                like_count AS "like_count!: i64",
                comment_count AS "comment_count!: i64"
            FROM filtered
            LIMIT $3 OFFSET $4
            "#,
            owner_id,
            requesting_user_id,
            limit,
            offset,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        req: &CreateNoteRequest,
        is_published: bool,
    ) -> Result<Note, sqlx::Error> {
        let note_type = req.note_type.as_ref().unwrap_or(&NoteType::Research);
        let research_fields = req.research_fields.clone().unwrap_or_default();
        let tags = req.tags.clone().unwrap_or_default();
        let published_at = if is_published {
            Some(chrono::Utc::now())
        } else {
            None
        };

        sqlx::query_as!(
            Note,
            r#"
            INSERT INTO notes (
                title, content, note_type, research_fields, tags,
                is_published, published_at, user_id, community_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id, title, content,
                note_type AS "note_type: NoteType",
                research_fields, tags, is_published,
                published_at, user_id, community_id,
                created_at, updated_at
            "#,
            req.title,
            req.content,
            note_type as _,
            &research_fields,
            &tags,
            is_published,
            published_at,
            user_id,
            req.community_id,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        note_id: Uuid,
        user_id: Uuid,
        req: &UpdateNoteRequest,
    ) -> Result<Option<Note>, sqlx::Error> {
        sqlx::query_as!(
            Note,
            r#"
            UPDATE notes SET
                title           = COALESCE($3, title),
                content         = COALESCE($4, content),
                note_type       = COALESCE($5, note_type),
                research_fields = COALESCE($6, research_fields),
                tags            = COALESCE($7, tags),
                is_published    = COALESCE($8, is_published),
                community_id    = COALESCE($9, community_id),
                updated_at      = NOW()
            WHERE id = $1 AND user_id = $2
            RETURNING
                id, title, content,
                note_type AS "note_type: NoteType",
                research_fields, tags, is_published,
                published_at, user_id, community_id,
                created_at, updated_at
            "#,
            note_id,
            user_id,
            req.title.as_deref(),
            req.content.as_deref(),
            req.note_type.as_ref() as _,
            req.research_fields.as_deref(),
            req.tags.as_deref(),
            req.is_published,
            req.community_id,
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, note_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"DELETE FROM notes WHERE id = $1 AND user_id = $2"#,
            note_id,
            user_id,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn toggle_like(
        pool: &PgPool,
        note_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        // Returns true = liked, false = unliked
        let existing = sqlx::query_scalar!(
            r#"SELECT id FROM note_likes WHERE note_id = $1 AND user_id = $2"#,
            note_id,
            user_id,
        )
        .fetch_optional(pool)
        .await?;

        if existing.is_some() {
            sqlx::query!(
                r#"DELETE FROM note_likes WHERE note_id = $1 AND user_id = $2"#,
                note_id,
                user_id,
            )
            .execute(pool)
            .await?;
            Ok(false)
        } else {
            sqlx::query!(
                r#"INSERT INTO note_likes (note_id, user_id) VALUES ($1, $2)"#,
                note_id,
                user_id,
            )
            .execute(pool)
            .await?;
            Ok(true)
        }
    }

    pub async fn create_images(
        pool: &PgPool,
        note_id: Uuid,
        image_urls: &[String],
    ) -> Result<(), sqlx::Error> {
        for (i, url) in image_urls.iter().enumerate() {
            sqlx::query!(
                r#"INSERT INTO note_images (note_id, image_url, "order") VALUES ($1, $2, $3)"#,
                note_id,
                url,
                i as i32,
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn replace_images(
        pool: &PgPool,
        note_id: Uuid,
        image_urls: &[String],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM note_images WHERE note_id = $1"#, note_id)
            .execute(pool)
            .await?;
        Self::create_images(pool, note_id, image_urls).await
    }

    pub async fn create_mentions(
        pool: &PgPool,
        note_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<(), sqlx::Error> {
        for user_id in user_ids {
            sqlx::query!(
                r#"
                INSERT INTO note_mentions (note_id, mentioned_user_id, position_start, position_end)
                VALUES ($1, $2, 0, 0)
                ON CONFLICT DO NOTHING
                "#,
                note_id,
                user_id,
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn sync_mentions(
        pool: &PgPool,
        note_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        // Returns newly added user IDs for notification purposes
        let existing: Vec<Uuid> = sqlx::query_scalar!(
            r#"SELECT mentioned_user_id FROM note_mentions WHERE note_id = $1"#,
            note_id,
        )
        .fetch_all(pool)
        .await?;

        let newly_added: Vec<Uuid> = user_ids
            .iter()
            .filter(|id| !existing.contains(id))
            .copied()
            .collect();

        let removed: Vec<Uuid> = existing
            .iter()
            .filter(|id| !user_ids.contains(id))
            .copied()
            .collect();

        if !removed.is_empty() {
            sqlx::query!(
                r#"
                DELETE FROM note_mentions
                WHERE note_id = $1 AND mentioned_user_id = ANY($2)
                "#,
                note_id,
                &removed,
            )
            .execute(pool)
            .await?;
        }

        Self::create_mentions(pool, note_id, &newly_added).await?;
        Ok(newly_added)
    }

    pub async fn is_community_member(
        pool: &PgPool,
        community_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM community_memberships
                WHERE community_id = $1 AND user_id = $2
            ) AS "exists!: bool"
            "#,
            community_id,
            user_id,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn count_by_user(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!: i64" FROM notes WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(pool)
        .await
    }
}
