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

use super::model::{MarkReadRequest, NotificationQuery};
use super::service::{NotificationError, NotificationService};

pub async fn list_notifications(
    State(state): State<SharedState>,
    claims: Claims,
    Query(query): Query<NotificationQuery>,
) -> impl IntoResponse {
    match NotificationService::list(
        &state.databases.db.pool,
        claims.id,
        query.r#type.as_deref(),
        query.read,
    )
    .await
    {
        Ok(result) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: result,
        }
        .into_response(),
        Err(e) => notif_error(e),
    }
}

pub async fn get_notification(
    State(state): State<SharedState>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match NotificationService::get_by_id(&state.databases.db.pool, id, claims.id).await {
        Ok(n) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: n,
        }
        .into_response(),
        Err(e) => notif_error(e),
    }
}

pub async fn mark_read(
    State(state): State<SharedState>,
    claims: Claims,
    Json(body): Json<MarkReadRequest>,
) -> impl IntoResponse {
    if body.notification_ids.is_empty() {
        return ApiResponse {
            status: "At least one notification ID is required".to_string(),
            code: StatusCode::BAD_REQUEST.as_u16(),
            data: Option::<()>::None,
        }
        .into_response();
    }

    match NotificationService::mark_read(
        &state.databases.db.pool,
        &body.notification_ids,
        claims.id,
        body.read,
    )
    .await
    {
        Ok(count) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: serde_json::json!({ "updated": count }),
        }
        .into_response(),
        Err(e) => notif_error(e),
    }
}

pub async fn delete_notification(
    State(state): State<SharedState>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match NotificationService::delete(&state.databases.db.pool, id, claims.id).await {
        Ok(_) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: serde_json::json!({ "message": "Notification deleted" }),
        }
        .into_response(),
        Err(e) => notif_error(e),
    }
}

pub async fn unread_count(State(state): State<SharedState>, claims: Claims) -> impl IntoResponse {
    match NotificationService::unread_count(&state.databases.db.pool, claims.id).await {
        Ok(count) => ApiResponse {
            status: "success".to_string(),
            code: StatusCode::OK.as_u16(),
            data: serde_json::json!({ "unread_count": count }),
        }
        .into_response(),
        Err(e) => notif_error(e),
    }
}

fn notif_error(e: NotificationError) -> axum::response::Response {
    match e {
        NotificationError::NotFound => ApiResponse {
            status: "Notification not found".to_string(),
            code: StatusCode::NOT_FOUND.as_u16(),
            data: Option::<()>::None,
        }
        .into_response(),
        NotificationError::DatabaseError(_) => ApiResponse {
            status: "Internal server error".to_string(),
            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            data: Option::<()>::None,
        }
        .into_response(),
    }
}
