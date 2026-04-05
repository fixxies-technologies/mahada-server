use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::app_state::SharedState;
use crate::common::response::ApiResponse;
use crate::security::jwt::Claims;

use super::model::{CreateNoteRequest, NoteByUserQuery, NoteQuery, UpdateNoteRequest};
use super::service::{NoteError, NoteService};

pub async fn list_notes(
    State(state): State<SharedState>,
    claims: Claims,
    Query(query): Query<NoteQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(10).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    match NoteService::list(
        &state.databases.db.pool,
        claims.id,
        limit,
        offset,
        query.search.as_deref(),
        query.community_id,
    )
    .await
    {
        Ok(result) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: result,
        }
        .into_response(),
        Err(e) => note_error(e),
    }
}

pub async fn list_notes_by_user(
    State(state): State<SharedState>,
    claims: Claims,
    Path(user_id): Path<Uuid>,
    Query(query): Query<NoteByUserQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(10).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    match NoteService::list_by_user(&state.databases.db.pool, user_id, claims.id, limit, offset)
        .await
    {
        Ok(result) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: result,
        }
        .into_response(),
        Err(e) => note_error(e),
    }
}

pub async fn get_note(
    State(state): State<SharedState>,
    claims: Claims,
    Path(note_id): Path<Uuid>,
) -> impl IntoResponse {
    match NoteService::get_by_id(&state.databases.db.pool, note_id, claims.id).await {
        Ok(note) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: note,
        }
        .into_response(),
        Err(e) => note_error(e),
    }
}

pub async fn create_note(
    State(state): State<SharedState>,
    claims: Claims,
    Json(body): Json<CreateNoteRequest>,
) -> impl IntoResponse {
    if body.title.trim().is_empty() || body.content.trim().is_empty() {
        return ApiResponse {
            status: "Title and content are required".to_string(),
            code: StatusCode::BAD_REQUEST.as_u16(),
            data: Option::<()>::None,
        }
        .into_response();
    }

    match NoteService::create(
        &state.databases.db.pool,
        &state.events.bus,
        claims.id,
        body,
        &claims.full_name,
    )
    .await
    {
        Ok(note) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::CREATED.as_u16(),
            data: note,
        }
        .into_response(),
        Err(e) => note_error(e),
    }
}

pub async fn update_note(
    State(state): State<SharedState>,
    claims: Claims,
    Path(note_id): Path<Uuid>,
    Json(body): Json<UpdateNoteRequest>,
) -> impl IntoResponse {
    match NoteService::update(
        &state.databases.db.pool,
        &state.events.bus,
        note_id,
        claims.id,
        body,
        &claims.full_name,
    )
    .await
    {
        Ok(note) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: note,
        }
        .into_response(),
        Err(e) => note_error(e),
    }
}

pub async fn delete_note(
    State(state): State<SharedState>,
    claims: Claims,
    Path(note_id): Path<Uuid>,
) -> impl IntoResponse {
    match NoteService::delete(&state.databases.db.pool, note_id, claims.id).await {
        Ok(_) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: serde_json::json!({ "message": "Note deleted successfully" }),
        }
        .into_response(),
        Err(e) => note_error(e),
    }
}

pub async fn toggle_like(
    State(state): State<SharedState>,
    claims: Claims,
    Path(note_id): Path<Uuid>,
) -> impl IntoResponse {
    match NoteService::toggle_like(
        &state.databases.db.pool,
        &state.events.bus,
        note_id,
        claims.id,
        &claims.full_name,
    )
    .await
    {
        Ok(liked) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: serde_json::json!({ "liked": liked }),
        }
        .into_response(),
        Err(e) => note_error(e),
    }
}

fn note_error(e: NoteError) -> axum::response::Response {
    match e {
        NoteError::NotFound => ApiResponse {
            status: "Note not found".to_string(),
            code: StatusCode::NOT_FOUND.as_u16(),
            data: Option::<()>::None,
        }
        .into_response(),
        NoteError::Unauthorized => ApiResponse {
            status: "Unauthorized".to_string(),
            code: StatusCode::UNAUTHORIZED.as_u16(),
            data: Option::<()>::None,
        }
        .into_response(),
        NoteError::NotCommunityMember => ApiResponse {
            status: "Not a member of this community".to_string(),
            code: StatusCode::FORBIDDEN.as_u16(),
            data: Option::<()>::None,
        }
        .into_response(),
        NoteError::DatabaseError(_) => ApiResponse {
            status: "Internal server error".to_string(),
            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            data: Option::<()>::None,
        }
        .into_response(),
        NoteError::EventError(error) => todo!(),
    }
}
