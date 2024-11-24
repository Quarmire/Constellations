use axum::Router;

use crate::spaceport::Spaceport;

mod handlers;
mod v0;

pub fn configure(spaceport: Spaceport) -> Router {
    Router::new().nest("/v0", v0::configure(spaceport))
}