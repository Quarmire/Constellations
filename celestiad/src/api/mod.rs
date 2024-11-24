use axum::Router;
use cozo::DbInstance;
use zenoh::Session;
use tower_http::trace::TraceLayer;

mod handlers;
mod v0;

pub fn configure(db: DbInstance, zenoh_session: Session) -> Router {
    Router::new()
        .with_state(db.clone())
        .with_state(zenoh_session.clone())
        .nest("/v0", v0::configure(db, zenoh_session))
        .layer(TraceLayer::new_for_http())
}