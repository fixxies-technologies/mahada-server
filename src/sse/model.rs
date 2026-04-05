use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct SseClient {
    pub user_id: Uuid,
    pub connection_id: String,
    pub channel: mpsc::UnboundedSender<Result<String, axum::Error>>,
    pub last_event_id: String,
    pub connected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    pub event: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
}

impl SseClient {
    pub fn new(
        user_id: Uuid,
        connection_id: String,
        channel: mpsc::UnboundedSender<Result<String, axum::Error>>,
        last_event_id: String,
    ) -> Self {
        Self {
            user_id,
            connection_id,
            channel,
            last_event_id,
            connected_at: Utc::now(),
        }
    }

    pub fn update_last_event_id(&mut self, event_id: String) {
        self.last_event_id = event_id;
    }
}
