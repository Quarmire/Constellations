use std::sync::Arc;

use crate::spaceport::Spaceport;

use super::handlers;
use axum::routing::{get, put};
use axum::Router;
use cozo::DbInstance;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use zenoh::Session;

pub fn configure(state: crate::State) -> Router {
    Router::new()
        .with_state(state.clone())
        .route("/hello", get(handlers::hello::hello))
        .route("/open", get(handlers::open::open).with_state(state.clone()))
        .route("/enable_transcription", get(handlers::enable_transcription::enable_transcription).with_state(state.clone()))
}