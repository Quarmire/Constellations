use std::{ops::DerefMut, sync::Arc, time::Duration};

use axum::{extract::{Path, State, Query}, http::StatusCode};
use cozo::DbInstance;
use serde::Deserialize;
use tokio::{sync::{mpsc::Sender, oneshot, Mutex}, task::JoinHandle};
use ulid::Ulid;
use zenoh::Session;

use crate::spaceport::Spaceport;

// A struct for query parameters
#[derive(Deserialize)]
pub struct SpaceportOpen {
    pub name: String,
}

pub async fn open(State(state): State<crate::State>, Query(open_query): Query<SpaceportOpen>) -> Result<String, StatusCode> {
    // let res = db.run_default("::relations").unwrap();
    // let agg: Vec<String> = res.into_iter().map(|x| {x[0].get_str().unwrap().to_string()}).collect();
    // let relations: &str = &agg.join(", ");

    let spaceport_name = open_query.name.clone();
    let state_copy = state.clone();

    let handle = tokio::spawn(async move {
        let (tx, rx) = oneshot::channel();
        state_copy.spaceport_tx.send(crate::SpaceportRequest::Open { name: spaceport_name, response: tx }).await;
        let s = rx.await.unwrap().unwrap();
        let session = s.docks.clone();
        Spaceport::serve_api(s, state_copy.spaceport_tx.clone(), state_copy.holobank_tx.clone(), state_copy, 9999).await;
    });

    let _ = state.clone().task_tx.send(handle).await;
   
    Ok(format!("{} spaceport's API can be accessed at localhost:{} \n", open_query.name, 9999).to_string())
}