use super::handler::sse_handler;
use crate::app_state::SharedState;
use crate::security::middlewares::jwt_middleware;
use axum::{Router, middleware, routing::get};

pub fn router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/", get(sse_handler))
        .route_layer(middleware::from_fn_with_state(state, jwt_middleware))
}
