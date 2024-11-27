use core::task;
use std::{ops::DerefMut, sync::Arc, time::Duration};

use axum::{extract::{Path, State, Query}, http::StatusCode};
use cozo::DbInstance;
use serde::Deserialize;
use tokio::{sync::{mpsc::{self, Sender}, oneshot, Mutex}, task::JoinHandle};
use tracing::{debug, info};
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

    handle_text_block_queries(user.name.clone(), state.session.clone(), h.clone(), spaceport_task_tx).await;

    let username_clone = user.name.clone();
    let celestiad_session_clone = state.celestiad_state.session.clone();
    let h_clone = h.clone();

    let handle = tokio::spawn(async move {
        let key_expression = "constellations/holobank?user=".to_string() + username_clone.as_str();
        let replies = celestiad_session_clone.get(key_expression.as_str()).await.unwrap();
        while let Ok(reply) = replies.recv_timeout(Duration::from_millis(100)) {
            let content = String::from_utf8(reply.unwrap().result().unwrap().payload().to_bytes().to_vec()).unwrap();
            info!("Exisiting user found; syncing holobanks.");
            h_clone.update_content(content);
        }
    });
    state.task_tx.send(handle).await;

    handle_text_content_relation_update_queries(user.name.clone(), state.celestiad_state.session.clone(), h.clone(), state.task_tx.clone()).await;
   
    let ids = h.get_text_block_ids();

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

async fn handle_text_content_relation_update_queries(user: String, session: Session, h: Holobank, task_tx: mpsc::Sender<JoinHandle<()>>) {
    let (tx, rx) = flume::bounded(32);
    let key_expression = "constellations/holobank";
    let handle = tokio::spawn(async move {
        let queryable = session
            .declare_queryable(key_expression)
            .callback(move |query| tx.send(query).unwrap())
            .await
            .unwrap();

        while let Ok(query) = rx.recv_async().await {
            debug!(">> Handling holobank query for user ({}): '{}'", user, query.selector());
            let username = query.parameters().get("user").unwrap_or("");
            if username.to_string() == user {
                let content = h.export_content();
                query.reply(key_expression, format!("{}", content)).await.unwrap();
            }
        }
    });

    task_tx.send(handle);
}