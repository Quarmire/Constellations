use core::task;
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
pub struct User {
    pub name: String,
}

pub async fn text(State(state): State<crate::spaceport::SpaceportState>, Query(user): Query<User>) -> Result<String, StatusCode> {
    let spaceport_task_tx = state.task_tx.clone();
    let (tx, rx) = oneshot::channel();
    state.holobank_tx.send(crate::HolobankRequest::GetHolobank { user: user.name.clone(), response: tx, db: state.celestiad_state.db.clone() }).await;
    let h = rx.await.unwrap().unwrap();

    let ids = h.get_text_block_ids();

    handle_text_block_queries(user.name.clone(), state.session.clone(), h, spaceport_task_tx).await;
   
    Ok(format!("{}", ids).to_string())
}

async fn handle_text_block_queries(user: String, session: Session, holobank: Holobank, task_tx: mpsc::Sender<JoinHandle<()>>) {
    let (tx, rx) = flume::bounded(32);
    let key_expression = user + "/holobank/block/text";
    let handle = tokio::spawn(async move {
        let queryable = session
            .declare_queryable(key_expression.as_str())
            .callback(move |query| tx.send(query).unwrap())
            .await
            .unwrap();

        while let Ok(query) = rx.recv_async().await {
            debug!(">> Handling text block query for '{}'", query.selector());
            let block_id = query.parameters().get("id").unwrap();
            let block_id = Ulid::from_string(&block_id).unwrap();

            let text = match holobank.get_text_block_content(block_id) {
                Some(content) => {content}
                None => {"".to_string()}
            };

            query.reply(key_expression.as_str(), format!("{}", text)).await.unwrap();
        }
    });

    task_tx.send(handle);
}