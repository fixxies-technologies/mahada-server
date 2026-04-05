use anyhow::Result;
use redis::RedisError;
use redis::aio::ConnectionManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

use super::model::{SseClient, StreamMessage};

const MAX_STREAM_LENGTH: usize = 10_000;
const PING_INTERVAL_SECS: u64 = 30;
const CLEANUP_INTERVAL_SECS: u64 = 3_600;
const ONLINE_STATUS_INTERVAL_SECS: u64 = 30;
const ONLINE_STATUS_TTL_SECS: u64 = 90;
const POLL_INTERVAL_MS: u64 = 200;
const STREAM_BATCH_SIZE: usize = 10;

pub struct SseManager {
    clients: Arc<RwLock<HashMap<Uuid, HashMap<String, Arc<RwLock<SseClient>>>>>>,
    redis: ConnectionManager,
}

impl SseManager {
    pub async fn new(redis: ConnectionManager) -> Arc<Self> {
        let manager = Arc::new(Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            redis,
        });

        manager.clone().spawn_ping_task();
        manager.clone().spawn_cleanup_task();
        manager.clone().spawn_online_status_task();

        tracing::info!("[SSE] Manager started");
        manager
    }

    pub async fn add_client(
        &self,
        user_id: Uuid,
        connection_id: String,
        last_event_id: Option<String>,
    ) -> (
        mpsc::UnboundedReceiver<Result<String, axum::Error>>,
        Arc<RwLock<SseClient>>,
    ) {
        let cursor = match last_event_id.filter(|id| !id.is_empty() && id != "$") {
            Some(id) => id,
            None => self.get_delivery_cursor(user_id).await,
        };

        let (tx, rx) = mpsc::unbounded_channel();
        let client = Arc::new(RwLock::new(SseClient::new(
            user_id,
            connection_id.clone(),
            tx,
            cursor.clone(),
        )));

        {
            let mut clients = self.clients.write().await;
            clients
                .entry(user_id)
                .or_insert_with(HashMap::new)
                .insert(connection_id.clone(), client.clone());
        }

        self.mark_user_online(user_id).await;

        let stream_key = Self::stream_key(user_id);
        let redis = self.redis.clone();
        let client_clone = client.clone();

        tokio::spawn(async move {
            Self::consume_stream(redis, client_clone, stream_key, user_id, cursor).await;
        });

        (rx, client)
    }

    pub async fn remove_client(&self, user_id: Uuid, connection_id: &str) -> bool {
        let mut clients = self.clients.write().await;
        let mut last_connection = false;

        if let Some(user_clients) = clients.get_mut(&user_id) {
            if let Some(client) = user_clients.remove(connection_id) {
                let last_event_id = client.read().await.last_event_id.clone();
                self.save_delivery_cursor(user_id, &last_event_id).await;
            }
            if user_clients.is_empty() {
                clients.remove(&user_id);
                self.mark_user_offline(user_id).await;
                last_connection = true;
            }
        }

        last_connection
    }

    pub async fn broadcast_to_user(
        &self,
        user_id: Uuid,
        event: &str,
        payload: serde_json::Value,
    ) -> Result<String> {
        let stream_key = Self::stream_key(user_id);
        let msg = StreamMessage {
            event: event.to_string(),
            data: payload,
            timestamp: chrono::Utc::now().timestamp(),
        };
        let msg_json = serde_json::to_string(&msg)?;

        let stream_id: String = redis::cmd("XADD")
            .arg(&stream_key)
            .arg("MAXLEN")
            .arg("~")
            .arg(MAX_STREAM_LENGTH)
            .arg("*")
            .arg("data")
            .arg(&msg_json)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e: RedisError| anyhow::anyhow!("Failed to write to stream: {}", e))?;

        tracing::info!(
            "[SSE] broadcast: user={} event={} stream_id={}",
            user_id,
            event,
            stream_id
        );
        Ok(stream_id)
    }

    // ── Key helpers ─────────────────────────────────────────────────────────

    fn stream_key(user_id: Uuid) -> String {
        format!("notifications:{}", user_id)
    }

    fn cursor_key(user_id: Uuid) -> String {
        format!("delivery:cursor:{}", user_id)
    }

    fn online_key(user_id: Uuid) -> String {
        format!("online:{}", user_id)
    }

    // ── Cursor persistence ───────────────────────────────────────────────────

    async fn get_delivery_cursor(&self, user_id: Uuid) -> String {
        let key = Self::cursor_key(user_id);
        let result: Result<Option<String>, RedisError> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut self.redis.clone())
            .await;

        result.ok().flatten().unwrap_or_else(|| "0-0".to_string())
    }

    async fn save_delivery_cursor(&self, user_id: Uuid, cursor: &str) {
        let key = Self::cursor_key(user_id);

        let _: Result<(), RedisError> = redis::cmd("SET")
            .arg(&key)
            .arg(cursor)
            .arg("EX")
            .arg(604_800u64)
            .query_async(&mut self.redis.clone())
            .await;
    }

    // ── Online status ────────────────────────────────────────────────────────

    async fn mark_user_online(&self, user_id: Uuid) {
        let key = Self::online_key(user_id);
        let _: Result<(), RedisError> = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("EX")
            .arg(ONLINE_STATUS_TTL_SECS)
            .query_async(&mut self.redis.clone())
            .await;
    }

    async fn mark_user_offline(&self, user_id: Uuid) {
        let key = Self::online_key(user_id);
        let _: Result<(), RedisError> = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.redis.clone())
            .await;
    }

    // ── Background tasks ─────────────────────────────────────────────────────

    fn spawn_ping_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(PING_INTERVAL_SECS));
            loop {
                interval.tick().await;
                let clients = self.clients.read().await;
                for user_clients in clients.values() {
                    for client in user_clients.values() {
                        let ping = "event: ping\ndata: {}\n\n".to_string();
                        let _ = client.read().await.channel.send(Ok(ping));
                    }
                }
            }
        });
    }

    fn spawn_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
            loop {
                interval.tick().await;
                let mut clients = self.clients.write().await;
                clients.retain(|_, user_clients| {
                    user_clients.retain(|_, client| {
                        if let Ok(c) = client.try_read() {
                            !c.channel.is_closed()
                        } else {
                            true // keep if we can't acquire the lock
                        }
                    });
                    !user_clients.is_empty()
                });
                tracing::info!("[SSE] cleanup: {} users connected", clients.len());
            }
        });
    }

    fn spawn_online_status_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(ONLINE_STATUS_INTERVAL_SECS));
            loop {
                interval.tick().await;
                let clients = self.clients.read().await;
                for user_id in clients.keys() {
                    self.mark_user_online(*user_id).await;
                }
            }
        });
    }

    // ── Stream consumer ───────────────────────────────────────────────────────

    async fn consume_stream(
        mut redis: ConnectionManager,
        client: Arc<RwLock<SseClient>>,
        stream_key: String,
        user_id: Uuid,
        initial_cursor: String,
    ) {
        let mut last_id = initial_cursor;
        tracing::info!(
            "[SSE] consume_stream started: user={} from={}",
            user_id,
            last_id
        );

        loop {
            if client.read().await.channel.is_closed() {
                tracing::info!("[SSE] channel closed: user={}", user_id);
                let key = Self::cursor_key(user_id);
                let _: Result<(), RedisError> = redis::cmd("SET")
                    .arg(&key)
                    .arg(&last_id)
                    .arg("EX")
                    .arg(604_800u64)
                    .query_async(&mut redis)
                    .await;
                break;
            }

            let result: Result<redis::Value, RedisError> = redis::cmd("XREAD")
                .arg("COUNT")
                .arg(STREAM_BATCH_SIZE)
                .arg("STREAMS")
                .arg(&stream_key)
                .arg(&last_id)
                .query_async(&mut redis)
                .await;

            match result {
                Ok(redis::Value::Nil) => {
                    // no new messages
                }
                Ok(redis::Value::Array(ref v)) if v.is_empty() => {
                    // no new messages
                }
                Ok(value) => {
                    if let Some(entries) = Self::parse_xread_reply(value) {
                        for (entry_id, data_str) in entries {
                            let stream_msg = match serde_json::from_str::<StreamMessage>(&data_str)
                            {
                                Ok(m) => m,
                                Err(e) => {
                                    tracing::warn!(
                                        "[SSE] deserialize error id={}: {}",
                                        entry_id,
                                        e
                                    );
                                    last_id = entry_id;
                                    continue;
                                }
                            };

                            let sse_line = format!(
                                "id: {}\nevent: {}\ndata: {}\n\n",
                                entry_id,
                                stream_msg.event,
                                serde_json::to_string(&stream_msg.data)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );

                            let sent = client.read().await.channel.send(Ok(sse_line)).is_ok();
                            if sent {
                                client.write().await.last_event_id = entry_id.clone();
                                last_id = entry_id;
                            } else {
                                tracing::info!(
                                    "[SSE] channel closed mid-delivery: user={}",
                                    user_id
                                );
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("[SSE] XREAD error: user={} err={}", user_id, e);
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }

            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    }

    fn parse_xread_reply(value: redis::Value) -> Option<Vec<(String, String)>> {
        let streams = match value {
            redis::Value::Array(v) => v,
            _ => return None,
        };
        let stream = streams.into_iter().next()?;
        let parts = match stream {
            redis::Value::Array(v) => v,
            _ => return None,
        };
        if parts.len() < 2 {
            return None;
        }
        let messages = match &parts[1] {
            redis::Value::Array(v) => v.clone(),
            _ => return None,
        };

        let mut out = Vec::new();
        for msg in messages {
            let msg_parts = match msg {
                redis::Value::Array(v) => v,
                _ => continue,
            };
            if msg_parts.len() < 2 {
                continue;
            }
            let entry_id = match &msg_parts[0] {
                redis::Value::BulkString(b) => String::from_utf8(b.clone()).ok()?,
                redis::Value::SimpleString(s) => s.clone(),
                _ => continue,
            };
            let fields = match &msg_parts[1] {
                redis::Value::Array(v) => v,
                _ => continue,
            };
            let mut i = 0;
            while i + 1 < fields.len() {
                let key = match &fields[i] {
                    redis::Value::BulkString(b) => String::from_utf8(b.clone()).ok(),
                    redis::Value::SimpleString(s) => Some(s.clone()),
                    _ => None,
                };
                if key.as_deref() == Some("data") {
                    let val = match &fields[i + 1] {
                        redis::Value::BulkString(b) => String::from_utf8(b.clone()).ok(),
                        redis::Value::SimpleString(s) => Some(s.clone()),
                        _ => None,
                    };
                    if let Some(v) = val {
                        out.push((entry_id.clone(), v));
                        break;
                    }
                }
                i += 2;
            }
        }
        Some(out)
    }
}
