use axum::{extract::{Path, State, Query}, http::StatusCode};
use local_ip_address::local_ip;

pub async fn enable_transcription(State(state): State<crate::State>) -> Result<String, StatusCode> {
    let state = state.clone();
    let handle = tokio::spawn(async move {
        let session = state.session.clone();
        let (tx, rx) = flume::bounded(32);
        session
            .declare_queryable("transcription/endpoint")
            .callback(move |query| tx.send(query).unwrap())
            .await
            .unwrap();
        // queryable run in background until the session is closed
        tokio::spawn(async move {
            while let Ok(query) = rx.recv_async().await {
                println!(">> Handling transcription endpoint query '{}'", query.selector());
                let addr = local_ip().unwrap().to_string();
                query.reply("transcription/endpoint", addr.as_str()).await.unwrap();
            }
        });
    });

    let _ = state.task_tx.send(handle).await;
   
    // Ok(format!("Relations: {}, {}\n", relations, open_query.name).to_string())
    Ok(format!("Enabled transcription endpoint lookup.\n").to_string())
}