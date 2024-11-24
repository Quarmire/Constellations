use super::handlers;
use axum::routing::{get, put};
use axum::Router;
use cozo::DbInstance;
use zenoh::Session;

pub fn configure(db: DbInstance, zenoh_session: Session) -> Router {
    Router::new()
        .with_state(db.clone())
        .with_state(zenoh_session)
        .route("/hello", get(handlers::hello::hello))
        .route("/open", get(handlers::open::open).with_state(db.clone()))
}