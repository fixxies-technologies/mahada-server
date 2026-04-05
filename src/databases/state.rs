use crate::databases::{pincecone::PineconeClient, postgres::DB, redis::Redis};

pub struct DatabaseState {
    pub db: DB,
    pub redis: Redis,
    pub pinecone: PineconeClient,
}

impl DatabaseState {
    pub async fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
        let pinecone_api_key =
            std::env::var("PINECONE_API_KEY").expect("PINECONE_API_KEY must be set");
        let pinecone_host = std::env::var("PINECONE_HOST").expect("PINECONE_HOST must be set");

        let db = DB::new(database_url, 10, 20, 30)
            .await
            .expect("Failed to connect to Postgres");

        let redis = Redis::new(&redis_url).expect("Failed to connect to Redis");

        let pinecone = PineconeClient::new(pinecone_api_key, pinecone_host);

        Self {
            db,
            redis,
            pinecone,
        }
    }
}
