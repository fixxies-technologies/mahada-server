use axum::{Router, middleware, routing::get};

use crate::app_state::SharedState;
use crate::security::middlewares::jwt_middleware;

use super::handler::{
    delete_notification, get_notification, list_notifications, mark_read, unread_count,
};

pub fn router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/", get(list_notifications))
        .route("/read", axum::routing::put(mark_read))
        .route("/unread-count", get(unread_count))
        .route("/:id", get(get_notification).delete(delete_notification))
        .route_layer(middleware::from_fn_with_state(state, jwt_middleware))
}
