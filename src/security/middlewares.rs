use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::time::Instant;
use uuid::Uuid;

use crate::app_state::SharedState;
use crate::common::response::ApiResponse;

pub async fn jwt_middleware(
    State(state): State<SharedState>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = match req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    {
        Some(token) => token,
        None => {
            return ApiResponse {
                code: StatusCode::UNAUTHORIZED.as_u16(),
                status: "Invalid or missing token".to_string(),
                data: Option::<()>::None,
            }
            .into_response();
        }
    };

    let claims = match state.security.keys.decode(token) {
        Ok(c) => c,
        Err(_) => {
            return ApiResponse {
                code: StatusCode::UNAUTHORIZED.as_u16(),
                status: "Invalid token".to_string(),
                data: Option::<()>::None,
            }
            .into_response();
        }
    };

    req.extensions_mut().insert(claims);
    next.run(req).await
}

pub async fn logging_middleware(mut req: Request<axum::body::Body>, next: Next) -> Response {
    let request_id = Uuid::new_v4();
    req.extensions_mut().insert(request_id);

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    let res = next.run(req).await;

    let status = res.status().as_u16();
    let latency = start.elapsed().as_millis();

    if status >= 500 {
        tracing::error!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            latency_ms = latency,
            "server error"
        );
    } else {
        tracing::info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            latency_ms = latency,
            "request completed"
        );
    }

    res
}
