use axum::{extract::{Path, State, Query}, http::StatusCode};
use cozo::DbInstance;
use serde::Deserialize;
use ulid::Ulid;

pub async fn dock() -> Result<String, StatusCode> {
    Ok(format!("Relations:").to_string())
}