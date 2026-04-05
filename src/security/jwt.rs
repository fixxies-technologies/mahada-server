use anyhow::Result;
use axum::{extract::FromRequestParts, http::request::Parts, response::IntoResponse};
use chrono::{TimeDelta, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::response::ApiResponse;

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub id: Uuid,
    pub last_name: String,
    pub first_name: String,
    pub full_name: String,
    pub username: String,
    pub exp: usize,
    pub sub: String,
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = std::result::Result<Self, Self::Rejection>> + Send {
        async move {
            parts.extensions.get::<Claims>().cloned().ok_or_else(|| {
                ApiResponse {
                    status: "Unauthorized".to_string(),
                    code: StatusCode::UNAUTHORIZED.as_u16(),
                    data: Option::<()>::None,
                }
                .into_response()
            })
        }
    }
}

pub struct Keys {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
}

impl Keys {
    pub fn new(encoding_key: EncodingKey, decoding_key: DecodingKey) -> Self {
        Self {
            encoding_key,
            decoding_key,
        }
    }

    pub fn encode(&self, claims: Claims) -> Result<String> {
        let exp = (Utc::now() + TimeDelta::try_minutes(30).expect("Invalid duration")).timestamp()
            as usize;

        let username = claims.username.clone();

        let claims = Claims {
            exp,
            sub: username.clone(),
            username,
            ..claims
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok(token)
    }

    pub fn decode(&self, token: &str) -> Result<Claims> {
        let data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        )?;

        Ok(data.claims)
    }
}
