use axum::{Router, middleware, routing::get};
use tokio::net::TcpListener;

use crate::app_state::SharedState;
use crate::security::middlewares::logging_middleware;

mod app_state;
mod common;
mod databases;
mod events;
mod note;
mod notification;
mod security;
mod sse;
mod user;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let state: SharedState = app_state::AppState::new().await;

    let router = Router::new()
        .route("/", get(async || "Mahada is live".to_string()))
        .nest("/notes", note::routes::router(state.clone()))
        .nest(
            "/notifications",
            notification::routes::router(state.clone()),
        )
        .nest("/sse", sse::routes::router(state.clone()))
        .with_state(state)
        .layer(middleware::from_fn(logging_middleware));

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::info!("[SERVER] Listening on {}", addr);
    axum::serve(listener, router).await.unwrap();

    Ok(())
}
