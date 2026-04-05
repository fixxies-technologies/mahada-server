use crate::sse::manager::SseManager;
use redis::aio::ConnectionManager;
use std::sync::Arc;

pub struct SseState {
    pub manager: Arc<SseManager>,
}

impl SseState {
    pub async fn new() -> Self {
        let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

        let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

        let conn_manager = ConnectionManager::new(client)
            .await
            .expect("Failed to create ConnectionManager");

        let manager = SseManager::new(conn_manager).await;

        Self { manager }
    }
}
