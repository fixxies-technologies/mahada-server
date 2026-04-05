use crate::events::bus::EventBus;

pub struct EventState {
    pub bus: EventBus,
}

impl EventState {
    pub async fn new() -> Self {
        let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

        let bus = EventBus::new(&redis_url)
            .await
            .expect("Failed to start EventBus");

        Self { bus }
    }
}
