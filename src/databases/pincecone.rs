use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorType {
    Note,
    Comment,
    PdfContent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: VectorType,
    pub content: String,
    pub user_id: String,
    pub post_id: String,
    pub parent_id: Option<String>,
    pub sentiment_score: Option<f64>,
    pub engagement_score: Option<f64>,
    pub title: Option<String>,
    pub research_fields: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct UpsertRecord {
    #[serde(rename = "_id")]
    pub id: String,
    pub chunk_text: String,
    #[serde(flatten)]
    pub metadata: VectorMetadata,
}

#[derive(Debug, Serialize)]
struct QueryRequest {
    query: QueryInput,
    top_k: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<serde_json::Value>,
    fields: Vec<String>,
}

#[derive(Debug, Serialize)]
struct QueryInput {
    inputs: QueryText,
}

#[derive(Debug, Serialize)]
struct QueryText {
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct QueryMatch {
    pub id: String,
    pub score: f32,
    pub fields: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub result: QueryResult,
    pub usage: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct QueryResult {
    pub hits: Vec<QueryMatch>,
}

pub struct PineconeClient {
    api_key: String,
    index_host: String,
    http: Client,
}

impl PineconeClient {
    pub fn new(api_key: String, index_host: String) -> Self {
        Self {
            api_key,
            index_host,
            http: Client::new(),
        }
    }

    pub async fn upsert_records(
        &self,
        records: Vec<UpsertRecord>,
        namespace: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!(
            "https://{}/records/namespaces/{}/upsert",
            self.index_host, namespace
        );

        let ndjson: String = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<String>>()
            .join("\n");

        self.http
            .post(&url)
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/x-ndjson")
            .header("X-Pinecone-Api-Version", "2025-10")
            .body(ndjson)
            .send()
            .await?;

        Ok(())
    }

    pub async fn query(
        &self,
        text: &str,
        namespace: &str,
        top_k: u32,
        filter: Option<serde_json::Value>,
    ) -> Result<QueryResponse, reqwest::Error> {
        let url = format!(
            "https://{}/records/namespaces/{}/search",
            self.index_host, namespace
        );

        let body = QueryRequest {
            query: QueryInput {
                inputs: QueryText {
                    text: text.to_string(),
                },
            },
            top_k,
            filter,
            fields: vec![
                "chunk_text".to_string(),
                "type".to_string(),
                "user_id".to_string(),
            ],
        };

        let response = self
            .http
            .post(&url)
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("X-Pinecone-Api-Version", "2025-10")
            .json(&body)
            .send()
            .await?
            .json::<QueryResponse>()
            .await?;

        Ok(response)
    }
}
