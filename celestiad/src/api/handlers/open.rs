use axum::{extract::{Path, State, Query}, http::StatusCode};
use cozo::DbInstance;
use serde::Deserialize;
use ulid::Ulid;

use crate::spaceport::Spaceport;

// A struct for query parameters
#[derive(Deserialize)]
pub struct SpaceportOpen {
    pub name: String,
}

pub async fn open(State(db): State<DbInstance>, Query(open_query): Query<SpaceportOpen>) -> Result<String, StatusCode> {
    let res = db.run_default("::relations").unwrap();
    let agg: Vec<String> = res.into_iter().map(|x| {x[0].get_str().unwrap().to_string()}).collect();
    let relations: &str = &agg.join(", ");
    let sp = Spaceport::open(Ulid::new()).await.unwrap();
    Ok(format!("Relations: {}, {}, {}\n", relations, open_query.name, sp.id).to_string())
}