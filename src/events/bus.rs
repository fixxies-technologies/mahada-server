use anyhow::Result;
use chrono::Utc;
use redis::{RedisError, aio::ConnectionManager};
use std::future::Future;
use std::pin::Pin;
use tokio::sync::watch;

use super::model::{Event, EventType};

const STREAM_NAME: &str = "events:app";
const MAX_STREAM_LEN: usize = 10_000;
const POLL_INTERVAL_MS: u64 = 200;
const BATCH_SIZE: usize = 50;
const CONSUMER_NAME: &str = "mahada-consumer";

pub type EventHandler =
    Box<dyn Fn(Event) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

pub struct EventBus {
    redis: ConnectionManager,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl EventBus {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let redis = ConnectionManager::new(client).await?;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Ok(Self {
            redis,
            shutdown_tx,
            shutdown_rx,
        })
    }

    pub async fn publish(&self, event: &Event) -> Result<String> {
        let data = serde_json::to_string(&event.payload)?;
        let event_type = serde_json::to_string(&event.event_type)?;
        let timestamp = event.timestamp.to_rfc3339();

        let stream_id: String = redis::cmd("XADD")
            .arg(STREAM_NAME)
            .arg("MAXLEN")
            .arg("~")
            .arg(MAX_STREAM_LEN)
            .arg("*")
            .arg("type")
            .arg(&event_type)
            .arg("data")
            .arg(&data)
            .arg("timestamp")
            .arg(&timestamp)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e: RedisError| anyhow::anyhow!("Failed to publish event: {}", e))?;

        Ok(stream_id)
    }

    pub async fn subscribe(
        &self,
        consumer_group: &str,
        event_types: Vec<EventType>,
        handler: EventHandler,
    ) -> Result<()> {
        let create_result: Result<(), RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(STREAM_NAME)
            .arg(consumer_group)
            .arg("$")
            .arg("MKSTREAM")
            .query_async(&mut self.redis.clone())
            .await;

        match create_result {
            Ok(_) => tracing::info!("[BUS] consumer group '{}' created", consumer_group),
            Err(e) if e.to_string().contains("BUSYGROUP") => {
                tracing::info!(
                    "[BUS] consumer group '{}' already exists, resuming",
                    consumer_group
                );
                Self::drain_pending(&mut self.redis.clone(), consumer_group).await;
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to create consumer group: {}", e)),
        }

        let redis = self.redis.clone();
        let shutdown_rx = self.shutdown_rx.clone();
        let group = consumer_group.to_string();

        tokio::spawn(async move {
            Self::consume_loop(redis, group, event_types, handler, shutdown_rx).await;
        });

        tracing::info!("[BUS] consumer task spawned for group '{}'", consumer_group);
        Ok(())
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    async fn consume_loop(
        mut redis: ConnectionManager,
        group: String,
        event_types: Vec<EventType>,
        handler: EventHandler,
        shutdown_rx: watch::Receiver<bool>,
    ) {
        let last_id = ">".to_string();

        loop {
            if *shutdown_rx.borrow() {
                tracing::info!("[BUS] shutting down consumer group '{}'", group);
                break;
            }

            // XREADGROUP to pull messages
            let results: Result<redis::Value, RedisError> = redis::cmd("XREADGROUP")
                .arg("GROUP")
                .arg(&group)
                .arg(CONSUMER_NAME)
                .arg("COUNT")
                .arg(BATCH_SIZE)
                .arg("BLOCK")
                .arg(POLL_INTERVAL_MS)
                .arg("STREAMS")
                .arg(STREAM_NAME)
                .arg(&last_id)
                .query_async(&mut redis)
                .await;

            let entries = match results {
                Ok(value) => Self::parse_stream_entries(value),
                Err(e) => {
                    tracing::error!("[BUS] XREADGROUP error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }
            };

            for (id, event) in entries {
                // Filter by event type if specified
                let should_handle = event_types.is_empty()
                    || event_types.iter().any(|t| {
                        std::mem::discriminant(t) == std::mem::discriminant(&event.event_type)
                    });

                if should_handle {
                    match handler(event).await {
                        Ok(_) => {
                            let _: Result<(), _> = redis::cmd("XACK")
                                .arg(STREAM_NAME)
                                .arg(&group)
                                .arg(&id)
                                .query_async(&mut redis)
                                .await;
                        }
                        Err(e) => {
                            tracing::error!("[BUS] handler error for id {}: {}", id, e);
                            // Don't ACK — message stays in PEL for retry
                        }
                    }
                } else {
                    // Not for this consumer — ACK to clear from PEL
                    let _: Result<(), _> = redis::cmd("XACK")
                        .arg(STREAM_NAME)
                        .arg(&group)
                        .arg(&id)
                        .query_async(&mut redis)
                        .await;
                }
            }
        }
    }

    /// Drain pending entries (PEL) from a crashed/restarted consumer group.
    async fn drain_pending(redis: &mut ConnectionManager, group: &str) {
        let result: Result<redis::Value, _> = redis::cmd("XPENDING")
            .arg(STREAM_NAME)
            .arg(group)
            .arg("-")
            .arg("+")
            .arg(100)
            .query_async(redis)
            .await;

        if let Ok(redis::Value::Array(entries)) = result {
            for entry in entries {
                if let redis::Value::Array(fields) = entry {
                    if let Some(redis::Value::BulkString(id_bytes)) = fields.first() {
                        let id = String::from_utf8_lossy(id_bytes).to_string();
                        let _: Result<(), _> = redis::cmd("XACK")
                            .arg(STREAM_NAME)
                            .arg(group)
                            .arg(&id)
                            .query_async(redis)
                            .await;
                    }
                }
            }
        }
    }

    fn parse_stream_entries(value: redis::Value) -> Vec<(String, Event)> {
        let mut out = Vec::new();

        let redis::Value::Array(streams) = value else {
            return out;
        };

        for stream in streams {
            let redis::Value::Array(mut stream_parts) = stream else {
                continue;
            };
            if stream_parts.len() < 2 {
                continue;
            }
            let redis::Value::Array(messages) = stream_parts.remove(1) else {
                continue;
            };

            for message in messages {
                let redis::Value::Array(mut parts) = message else {
                    continue;
                };
                if parts.len() < 2 {
                    continue;
                }

                let id = match parts.remove(0) {
                    redis::Value::BulkString(b) => String::from_utf8_lossy(&b).to_string(),
                    _ => continue,
                };

                let redis::Value::Array(fields) = parts.remove(0) else {
                    continue;
                };

                // fields is [key, value, key, value, ...]
                let mut map = std::collections::HashMap::new();
                let mut iter = fields.into_iter();
                while let (Some(k), Some(v)) = (iter.next(), iter.next()) {
                    if let (redis::Value::BulkString(kb), redis::Value::BulkString(vb)) = (k, v) {
                        map.insert(
                            String::from_utf8_lossy(&kb).to_string(),
                            String::from_utf8_lossy(&vb).to_string(),
                        );
                    }
                }

                let Some(type_str) = map.get("type") else {
                    continue;
                };
                let Some(data_str) = map.get("data") else {
                    continue;
                };

                let Ok(event_type) = serde_json::from_str::<EventType>(type_str) else {
                    continue;
                };
                let Ok(payload) = serde_json::from_str::<serde_json::Value>(data_str) else {
                    continue;
                };

                let timestamp = map
                    .get("timestamp")
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                out.push((
                    id.clone(),
                    Event {
                        id,
                        event_type,
                        payload,
                        timestamp,
                    },
                ));
            }
        }

        out
    }
}
