use anyhow::Result;
use sqlx::{PgPool, pool::PoolOptions};
use std::time::Duration;

pub struct DB {
    pub pool: PgPool,
}

impl DB {
    pub async fn new(
        conn: String,
        min_conn: u32,
        max_conn: u32,
        idle_timeout: u64,
    ) -> Result<Self> {
        let pool = PoolOptions::new()
            .min_connections(min_conn)
            .max_connections(max_conn)
            .idle_timeout(Some(Duration::from_secs(idle_timeout)))
            .connect(&conn)
            .await?;

        Ok(Self { pool })
    }
}
