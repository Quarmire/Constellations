use crate::spaceport::Spaceport;

use super::handlers;
use axum::routing::get;
use axum::Router;

pub fn configure(state: crate::spaceport::SpaceportState) -> Router {
    Router::new()
        .with_state(state.clone())
        .route("/dock", get(handlers::dock::dock).with_state(state.clone()))
        .route("/text", get(handlers::text::text).with_state(state.clone()))
        .route("/newtext", get(handlers::newtext::newtext).with_state(state.clone()))
        .route("/edittext", get(handlers::edittext::edittext).with_state(state.clone()))
        .route("/realtimetext", get(handlers::realtimetext::realtimetext).with_state(state.clone()))
}