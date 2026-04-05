use axum::{Router, middleware, routing::get};

use crate::app_state::SharedState;
use crate::security::middlewares::jwt_middleware;

use super::handler::{
    create_note, delete_note, get_note, list_notes, list_notes_by_user, toggle_like, update_note,
};

pub fn router(state: SharedState) -> Router<SharedState> {
    Router::new()
        // GET  /notes          — paginated list
        // POST /notes          — create
        .route("/", get(list_notes).post(create_note))
        // GET    /notes/:id    — fetch one
        // PATCH  /notes/:id    — update
        // DELETE /notes/:id    — delete
        .route("/:id", get(get_note).patch(update_note).delete(delete_note))
        // POST /notes/:id/like — toggle like
        .route("/:id/like", axum::routing::post(toggle_like))
        // GET /notes/user/:user_id — notes by a specific user
        .route("/user/:user_id", get(list_notes_by_user))
        .route_layer(middleware::from_fn_with_state(state, jwt_middleware))
}
