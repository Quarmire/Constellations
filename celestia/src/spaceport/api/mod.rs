use axum::Router;

mod handlers;
mod v0;

pub fn configure() -> Router {
    Router::new().nest("/v0", v0::configure())
}