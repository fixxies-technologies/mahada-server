use std::sync::Arc;

use crate::databases::state::DatabaseState;
use crate::events::state::EventState;
use crate::security::state::SecurityState;
use crate::sse::state::SseState;

pub struct AppState {
    pub databases: DatabaseState,
    pub events: EventState,
    pub sse: SseState,
    pub security: SecurityState,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub async fn new() -> SharedState {
        Arc::new(Self {
            databases: DatabaseState::new().await,
            events: EventState::new().await,
            sse: SseState::new().await,
            security: SecurityState::new(),
        })
    }
}
