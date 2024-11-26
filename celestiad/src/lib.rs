pub mod commands;
pub mod settings;
pub mod api;
pub mod spaceport;
pub mod holobank;

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

use cozo::{DataValue, DbInstance, ScriptMutability, UlidWrapper};
use holobank::Holobank;
use spaceport::Spaceport;
use tokio::sync::oneshot;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{debug, info};
use ulid::Ulid;
use anyhow::{anyhow, Result};

use simple_transcribe_rs::model_handler;
use simple_transcribe_rs::transcriber;
use zenoh::Session;

pub const CELESTIAD_HOST_DIR: &str = "/etc/constellations/";
pub const CELESTIAD_DATA_DIR: &str = "/var/lib/constellations/";
pub const CELESTIAD_HOLOBANK_DIR: &str = "/var/lib/constellations/holobank/";
pub const CELESTIAD_AUDIO_DIR: &str = "/var/lib/constellations/audio/";
pub const CELESTIAD_MODEL_DIR: &str = "/var/lib/constellations/models/";
pub const CELESTIAD_USER_DIR: &str = ".constellations/";
pub const CELESTIAD_ENV_PREFIX: &str = "CELESTIA";

pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const ALL: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
pub const TCP_ENDPOINT: &str = "tcp/[::]:7447";
pub const UDP_ENDPOINT: &str = "udp/[::]:7447";
pub const CELESTIAD_PORT: u16 = 9494;

pub const HOST_SCHEMA: &str = "
    :create host {
        id: Ulid,
        =>
        self: Bool,
    }
";

pub const SPACEPORT_SCHEMA: &str = "
    :create spaceport {
        name: String,
        =>
        id: Ulid,
        path: String,
        open: Bool,
    }
";

pub const HOLOBANK_SCHEMA: &str = "
    :create holobank {
        user: String,
        =>
        id: Ulid,
        path: String,
    }
";

pub enum HolobankRequest {
    GetHolobank{ user: String, response: oneshot::Sender<Result<Holobank, cozo::Error>>, db: DbInstance } // will get by ULID in the future
}

pub enum SpaceportRequest {
    Open{ name: String, response: oneshot::Sender<Result<Spaceport>> },
}

#[derive(Clone)]
pub struct State {
    pub session: Session,
    pub db: DbInstance,
    pub task_tx: mpsc::Sender<JoinHandle<()>>,
    pub llm_addr: Option<SocketAddr>,
    pub spaceport_tx: mpsc::Sender<SpaceportRequest>,
    pub holobank_tx: mpsc::Sender<HolobankRequest>,
}

pub async fn process_join_handles(mut rx: mpsc::Receiver<JoinHandle<()>>) {
    while let Some(handle) = rx.recv().await {
        match handle.await {
            Ok(_) => {
                println!("Task completed successfully");
            }
            Err(e) => {
                eprintln!("Task failed: {:?}", e);
            }
        }
    }
}

pub async fn process_holobank_requests(mut rx: mpsc::Receiver<HolobankRequest>) {
    let mut holobanks: BTreeMap<Ulid, Holobank> = BTreeMap::new();
    while let Some(request) = rx.recv().await {
        match request {
            HolobankRequest::GetHolobank { user, response, db } => {
                holobanks = handle_holobank(user, response, holobanks, db);
            }
        }
    }
}

pub async fn process_spaceport_requests(mut rx: mpsc::Receiver<SpaceportRequest>, state: State) {
    while let Some(request) = rx.recv().await {
        match request {
            SpaceportRequest::Open { name, response } => {
                handle_open_spaceport(name, response, state.db.clone()).await;
            }
        }
    }
}

async fn handle_open_spaceport(name: String, response: oneshot::Sender<Result<Spaceport>>, db: DbInstance) {
    let mut parameters = BTreeMap::new();
    parameters.insert("name".to_string(), DataValue::Str(name.clone().into()));
    match db.run_script(
        "?[a,b] := *spaceport{id: a, open: b, name: $name}",
        parameters.clone(),
        ScriptMutability::Mutable
    ) {
        Ok(rows) => { // Spaceport does not exist
            if rows.rows.len() < 1 {
                let new_id = Ulid::new();
                let s = Spaceport::open(new_id).await;
                response.send(s);
            }
            else if !rows.rows[0][1].get_bool().unwrap() {
                let id = rows.rows[0][0].get_ulid().unwrap();
                let s = Spaceport::open(id).await;
                response.send(s);
            }
            else {
                response.send(Err(anyhow!("Spaceport {} already open", name)));
            }
        },
        Err(e) => {
            response.send(Err(anyhow!("Could not open spaceport {}: {}", name, e)));
        },
    }
}

fn handle_holobank(user: String, response: oneshot::Sender<Result<Holobank, cozo::Error>>, mut holobanks: BTreeMap<Ulid, Holobank>, db: DbInstance) -> BTreeMap<Ulid, Holobank> {
    let mut parameters = BTreeMap::new();
    parameters.insert("user".to_string(), DataValue::Str(user.clone().into()));
    match db.run_script(
        "?[a,b] := *holobank{id: a, path: b, user: $user}",
        parameters.clone(),
        ScriptMutability::Mutable
    ) {
        Ok(rows) => {
            if rows.rows.len() < 1 { // Holobank does not exist for specified user
                let path = CELESTIAD_HOLOBANK_DIR.to_string() + user.clone().as_str();
                let path = Path::new(path.as_str());
                let new_id = Ulid::new();
                parameters.insert("path".to_string(), DataValue::Str(path.to_str().unwrap().into()));
                parameters.insert("id".to_string(), DataValue::Ulid(UlidWrapper(new_id)));
                let _ = db.run_script(
                    "?[id, user, path] <- [[$id, $user, $path]] :put holobank {id, user, path}",
                    parameters,
                    ScriptMutability::Mutable
                );
                let holobank = Holobank::load(path).unwrap();
                holobanks.insert(new_id, holobank.clone());
                debug!("Holobank does not exist yet.");
                response.send(Ok(holobank));
            }
            else {
                let id = rows.rows[0][0].get_ulid().unwrap();
                if holobanks.contains_key(&id) {
                        debug!("Holobank already loaded.");
                        response.send(Ok(holobanks.get(&id).unwrap().clone()));
                }
                else {
                    let path = Path::new(rows.rows[0][1].get_str().unwrap());
                    let holobank = Holobank::load(path).unwrap();
                    holobanks.insert(id, holobank.clone());
                    debug!("Holobank exists but not loaded yet.");
                    response.send(Ok(holobank));
                }
            }
        },
        Err(e) => {
            debug!("Error querying holobank relation: {}", e);
            response.send(Err(e));
        }
    };
    holobanks
}

pub async fn transcribe(audio_file: &str) {
    let m = model_handler::ModelHandler::new("tiny", CELESTIAD_MODEL_DIR).await;
    let trans = transcriber::Transcriber::new(m);
    let result = trans.transcribe((CELESTIAD_AUDIO_DIR.to_string() + audio_file + ".wav").as_str(), None).unwrap();
    let text = result.get_text();
    let start = result.get_start_timestamp();
    let end = result.get_end_timestamp();
    println!("start[{}]-end[{}] {}", start, end, text);
}

// let sp_db = cozo::DbInstance::new("sqlite", format!("{}/test/test.sqlite", root), "").unwrap();
// let mut parameters = BTreeMap::new();
// parameters.insert("name".to_string(), DataValue::Str(name.into()));
// let id = match Spaceport::setup_sp_db(&sp_db, crate::holobank::schema::SPACEPORT_SCHEMA) {
//     Ok(_) => {
//         let new_id = Ulid::new();
//         parameters.insert("id".to_string(), DataValue::Ulid(UlidWrapper(new_id)));
//         sp_db.run_script(
//             "?[id, name] <- [[$id, $name]] :put spaceport {id => name}",
//             parameters,
//             ScriptMutability::Mutable
//         );
//         new_id
//     },
//     Err(_) => {
//         let result = sp_db.run_script(
//             "?[a] := *spaceport{id: a, name: $name}",
//             parameters,
//             ScriptMutability::Mutable
//         ).unwrap();
//         let id = result.rows[0][0].clone().get_ulid().unwrap();
//         debug!("Database already exists for spaceport: {} (ID: {})", name, id);
//         id
//     }
// };