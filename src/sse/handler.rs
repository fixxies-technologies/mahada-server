use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Sse, sse::Event},
};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

use crate::app_state::SharedState;
use crate::security::jwt::Claims;

#[derive(Deserialize)]
pub struct SseQuery {
    pub last_event_id: Option<String>,
}

pub async fn sse_handler(
    State(state): State<SharedState>,
    claims: Claims,
    Query(query): Query<SseQuery>,
) -> impl IntoResponse {
    let connection_id = Uuid::new_v4().to_string();
    let user_id = claims.id;

    let (rx, _client) = state
        .sse
        .manager
        .add_client(user_id, connection_id.clone(), query.last_event_id)
        .await;

    let guard = DisconnectGuard {
        user_id,
        connection_id,
        manager: state.sse.manager.clone(),
    };

    let stream = UnboundedReceiverStream::new(rx)
        .map(|msg| match msg {
            Ok(raw) => Ok::<Event, std::convert::Infallible>(Event::default().data(raw)),
            Err(_) => Ok::<Event, std::convert::Infallible>(
                Event::default().event("error").data("stream error"),
            ),
        })
        .chain(futures_util::stream::once(async move {
            drop(guard);
            Ok::<Event, std::convert::Infallible>(Event::default().event("close").data(""))
        }));

    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("ping"),
        )
        .into_response()
}

struct DisconnectGuard {
    user_id: Uuid,
    connection_id: String,
    manager: Arc<crate::sse::manager::SseManager>,
}

impl Drop for DisconnectGuard {
    fn drop(&mut self) {
        let manager = self.manager.clone();
        let user_id = self.user_id;
        let conn_id = self.connection_id.clone();
        tokio::spawn(async move {
            manager.remove_client(user_id, &conn_id).await;
            tracing::info!("[SSE] disconnected: user={} conn={}", user_id, conn_id);
        });
    }
}
