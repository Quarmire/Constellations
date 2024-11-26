use std::sync::Arc;

use axum::Router;
use cozo::DbInstance;
use tokio::{sync::{mpsc::Sender, Mutex}, task::JoinHandle};
use zenoh::Session;
use tower_http::trace::TraceLayer;


mod handlers;
mod v0;

pub fn configure(state: crate::State) -> Router {
    Router::new()
        .with_state(state.clone())
        .nest("/v0", v0::configure(state.clone()))
        .layer(TraceLayer::new_for_http())
}