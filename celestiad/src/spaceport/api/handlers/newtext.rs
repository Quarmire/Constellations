use std::{ops::DerefMut, sync::Arc, time::Duration};

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

pub async fn newtext(State(state): State<crate::spaceport::SpaceportState>, Query(block): Query<Block>) -> Result<String, StatusCode> {
    let spaceport_task_tx = state.task_tx.clone();

    let (tx, rx) = oneshot::channel();
    state.holobank_tx.send(crate::HolobankRequest::GetHolobank { user: block.user.clone(), response: tx, db: state.celestiad_state.db.clone() }).await;
    let h = rx.await.unwrap().unwrap();

    let id = Ulid::from_string(block.id.as_str()).unwrap();

    h.set_text_block_content(id, "".to_string());

    handle_text_block_updates(block.user.clone(), state.session.clone(), h, id, spaceport_task_tx).await;

    Ok(format!("{}'s holobank subscribed to text block id: {}", block.user, block.id).to_string())
}

async fn handle_text_block_updates(user: String, session: Session, holobank: Holobank, id: Ulid, task_tx: mpsc::Sender<JoinHandle<()>>) {
    let key_expression = user + "/holobank/block/text/" + id.to_string().as_str();
    let subscriber = session.declare_subscriber(key_expression.as_str()).await.unwrap();
    
    let handle = tokio::spawn(async move {
        while let Ok(sample) = subscriber.recv_async().await {
            let content = String::from_utf8(sample.payload().to_bytes().to_vec()).unwrap();
            holobank.set_text_block_content(id, content);
        };
    });

    task_tx.send(handle).await;
}