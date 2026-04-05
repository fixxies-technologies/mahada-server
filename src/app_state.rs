use std::sync::Arc;

use crate::databases::state::DatabaseState;
use crate::events::model::EventType;
use crate::events::state::EventState;
use crate::notification::service::NotificationService;
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
        let databases = DatabaseState::new().await;
        let events = EventState::new().await;
        let sse = SseState::new().await;
        let security = SecurityState::new();

        let state = Arc::new(Self {
            databases,
            events,
            sse,
            security,
        });

        state.register_consumers().await;

        state
    }

    async fn register_consumers(&self) {
        self.register_notification_consumer().await;
    }

    async fn register_notification_consumer(&self) {
        let pool = self.databases.db.pool.clone();
        let sse = self.sse.manager.clone();

        let _ = self
            .events
            .bus
            .subscribe(
                "notifications-consumer",
                vec![
                    EventType::NoteMentioned,
                    EventType::NoteLiked,
                    EventType::NoteCommented,
                    EventType::UserFollowed,
                ],
                Box::new(move |event| {
                    let pool = pool.clone();
                    let sse = sse.clone();
                    Box::pin(async move {
                        NotificationService::handle_event(
                            &pool,
                            &sse,
                            &event.event_type,
                            &event.payload,
                        )
                        .await
                        .map_err(|e| {
                            tracing::error!("[NOTIFICATIONS] handler error: {}", e);
                            e
                        })
                    })
                }),
            )
            .await;
    }
}
