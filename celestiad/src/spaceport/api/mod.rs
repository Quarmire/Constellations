use axum::Router;
use tower_http::trace::TraceLayer;

use crate::spaceport::Spaceport;

mod handlers;
mod v0;

pub fn configure(state: crate::spaceport::SpaceportState) -> Router {
    Router::new()
        .with_state(state.clone())
        .nest("/v0", v0::configure(state))
        .layer(TraceLayer::new_for_http())
}