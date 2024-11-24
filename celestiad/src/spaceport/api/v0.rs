use crate::spaceport::Spaceport;

use super::handlers;
use axum::routing::get;
use axum::Router;

pub fn configure(spaceport: Spaceport) -> Router {
    Router::new()
        .with_state(spaceport)
        .route("/dock", get(handlers::dock::dock))
}