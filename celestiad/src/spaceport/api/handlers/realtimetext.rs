use std::{ops::{Deref, DerefMut}, sync::Arc, time::Duration};

use axum::{extract::{Path, State, Query}, http::StatusCode};
use cozo::DbInstance;
use serde::Deserialize;
use tokio::{sync::{mpsc::{self, Sender}, oneshot, Mutex}, task::JoinHandle};
use tracing::debug;
use ulid::Ulid;
use zenoh::Session;

use crate::{holobank::{self, Holobank}, spaceport::Spaceport};

#[derive(Deserialize)]
pub struct Block {
    pub user: String,
    pub id: String,
}

pub async fn realtimetext(State(state): State<crate::spaceport::SpaceportState>, Query(block): Query<Block>) -> Result<String, StatusCode> {
    let spaceport_task_tx = state.task_tx.clone();

    let id = Ulid::from_string(block.id.as_str()).unwrap();

    handle_realtime_text_block_updates(block.user.clone(), state.session.clone(), state.celestiad_state.session.clone(), id, spaceport_task_tx).await;

    Ok(format!("{}'s holobank subscribed to text block id: {}", block.user, block.id).to_string())
}

async fn handle_realtime_text_block_updates(user: String, spaceport_session: Session, celestiad_session: Session, id: Ulid, task_tx: mpsc::Sender<JoinHandle<()>>) {
    // let sub_key_expression = user.clone() + "/realtime/block/text/" + id.to_string().as_str();
    // let pub_key_expression = user + "/realtime/block/text/" + id.to_string().as_str();
    // let out_subscriber = spaceport_session.declare_subscriber(sub_key_expression.as_str()).await.unwrap();
    // let out_publisher = celestiad_session.declare_publisher(pub_key_expression.clone()).await.unwrap();
    // let in_subscriber = celestiad_session.declare_subscriber(sub_key_expression.as_str()).await.unwrap();
    // let in_publisher = spaceport_session.declare_publisher(pub_key_expression).await.unwrap();
    
    // let handle_out = tokio::spawn(async move {
    //     while let Ok(sample) = out_subscriber.recv_async().await {
    //         out_publisher.put(sample.payload().to_bytes()).await.unwrap();
    //     };
    // });

    // let handle_in = tokio::spawn(async move {
    //     while let Ok(sample) = in_subscriber.recv_async().await {
    //         in_publisher.put(sample.payload().to_bytes()).await.unwrap();
    //     };
    // });

    // task_tx.send(handle_in).await;
    // task_tx.send(handle_out).await;
}