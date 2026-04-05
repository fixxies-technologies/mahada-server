use anyhow::Result;
use redis::{AsyncTypedCommands, Client, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};

pub struct Redis {
    pub client: Client,
}

impl Redis {
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        Ok(Self { client })
    }

    async fn conn(&self) -> Result<MultiplexedConnection> {
        let conn = self.client.get_multiplexed_async_connection().await?;
        Ok(conn)
    }

    pub async fn set(&self, key: &str, value: &str, ttl_secs: Option<u64>) -> Result<()> {
        let mut conn = self.conn().await?;
        match ttl_secs {
            Some(ttl) => conn.set_ex(key, value, ttl).await?,
            None => conn.set(key, value).await?,
        }
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.conn().await?;
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    pub async fn del(&self, keys: &[&str]) -> Result<()> {
        let mut conn = self.conn().await?;
        conn.del(keys).await?;
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.conn().await?;
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    pub async fn expire(&self, key: &str, ttl_secs: u64) -> Result<()> {
        let mut conn = self.conn().await?;
        conn.expire(key, ttl_secs as i64).await?;
        Ok(())
    }

    pub async fn set_json<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: Option<u64>,
    ) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.set(key, &json, ttl_secs).await
    }

    pub async fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        match self.get(key).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }
}
