mod schema;
mod api;

use std::{fmt::Display, fs::File, io::{BufReader, BufWriter}, net::{IpAddr, Ipv4Addr, SocketAddr}};

use constellations_data::asset::{Asset, AssetError, Holographable, Materializable};
use serde::{Deserialize, Serialize};
use ulid::Ulid;
use chrono::{DateTime, Utc};

use std::{collections::{BTreeMap, HashMap}, io};
use cozo::{DataValue, DbInstance, ScriptMutability, UlidWrapper};
use tracing::{self, debug, info};
use tracing_subscriber::FmtSubscriber;
use tokio;

use zenoh::{self, config::{WhatAmI, ZenohId}, Config, Session};

use crate::settings::Settings;


// Provides services (data, communication, AI)
// HTTP for management via web ui, cli, tui, or native ui (tonic or hyper)
// Stores keys and configs in user home directory inside the .constellations folder.
// ~/.constellations/quarmire - commander directory; keys live here
// ~/.constellations/salvador - spaceport directory; configs live here
// ~/.constellations/frontier - another spaceport
// ~/.constellations/frontier/llama3 - service directory; service config lives here
// ~/.constellations/quarmire/holobank - holobank directory; dbinstance lives here
// ~/.constellations/quarmire/celestia-tui-seeker01 - spacecraft directory; application state lives here
// ~/.constellations/iro-01/ - starport

// Executed as command:
// spaceportd {home_directory} {commander_name}
// All spaceports are open by default
// If commander private keys are not in commander folder, password authentication is required.
// Spaceportd is running.  The API can be accessed via localhost:5425

// CLI:
// spaceport list - lists spaceports on device (opened and closed) (option --system: lists open spaceports in network)
// spaceport open [name]
// spaceport close [name]
// spaceport facilities [name]

// Future IPC: iceoryx2 + zenoh
// Current IPC: zenoh

const ROOT_DIR: &str = "/home/pmle/.constellations";
const TCP_ENDPOINT: &str = "tcp/[::]:0";
const UDP_ENDPOINT: &str = "udp/[::]:0";

pub struct Spaceport {
    // Designed around a zenoh session
    // multiple spaceports on one celestial body is possible
    // use hyperrail to communicate between local spaceports

    // has means to discover (scout) and establish
    // (auto config) comms with other spaceports

    // Has two means of communication: fast messages (radio) or
    // large payloads (ships)
    id: Ulid,
    name: String,
    db: DbInstance,
    radio: zenoh::Session, // external comms
    docks: zenoh::Session, // internal comms
    banks: Option<HashMap<String, HoloBank>>,
}

impl Spaceport {
    /// Opens spaceport by name.  Creates new spaceport if named spaceport does not exist.
    pub async fn open(name: &str, port: u16, settings: &Settings) -> Spaceport {
        let root = settings.root.directory.clone().unwrap();
        println!("{}", root);

        let sp_db = cozo::DbInstance::new("sqlite", format!("{}/test/test.sqlite", root), "").unwrap();
        let mut parameters = BTreeMap::new();
        parameters.insert("name".to_string(), DataValue::Str(name.into()));
        let id = match Spaceport::setup_sp_db(&sp_db, schema::SPACEPORT_SCHEMA) {
            Ok(_) => {
                let new_id = Ulid::new();
                parameters.insert("id".to_string(), DataValue::Ulid(UlidWrapper(new_id)));
                sp_db.run_script(
                    "?[id, name] <- [[$id, $name]] :put spaceport {id => name}",
                    parameters,
                    ScriptMutability::Mutable
                );
                new_id
            },
            Err(_) => {
                let result = sp_db.run_script(
                    "?[a] := *spaceport{id: a, name: $name}",
                    parameters,
                    ScriptMutability::Mutable
                ).unwrap();
                let id = result.rows[0][0].clone().get_ulid().unwrap();
                debug!("Database already exists for spaceport: {} (ID: {})", name, id);
                id
            }
        };

        let radio = Spaceport::open_radio(&id).await.unwrap();
        let docks = Spaceport::open_docks(&id,&name).await.unwrap();
        Spaceport::serve_api(port).await;

        // zenoh id = spaceport id

        // System id discovery:
        // scout for starports (zenoh routers)
        // if starport is found; get star system name
        // else scout for spaceports (zenoh peers)
        // if spaceport is found; get system id
        // else generate system ulid (time of discovery encoded)
        // this id is always the oldest active spaceport in the system
        // does not tie system id to network name, i.e., Wi-Fi SSID

        // Commander spacecraft:
        // stores commander id matched with username and personal info
        // stores biometric info
        // provides authentication service
        // peer nodes query the network for commander id using username + birthday
        // commander id is embedded in spacecraft and spaceports

        Spaceport {
            id,
            name: name.to_string(),
            db: sp_db,
            radio,
            docks,
            banks: Some(HashMap::new()),
        }
    }
    /// Close all docks and put facilities into hibernation.
    pub async fn close(&self) {
        todo!()
    }

    fn setup_sp_db(db: &DbInstance, schema: &str) -> Result<(), cozo::Error> {
        let result = db.run_default(schema);
        match result {
            Err(e) => {
                info!("Error: {}", e);
                Err(e)
            },
            Ok(_) => {
                info!("Created new spaceport database.");
                Ok(())
            }
        }
    }

    async fn open_docks(dock_id: &Ulid, spaceport_name: &str) -> io::Result<Session> {
        let mut config = zenoh::Config::default();
        config.set_id(ZenohId::try_from(dock_id.to_bytes().as_slice()).unwrap()).unwrap();
        config.set_mode(Some(WhatAmI::Router));
        config.scouting.multicast.set_enabled(Some(false));
        config.scouting.gossip.set_enabled(Some(false));
        config.transport.link.set_protocols(Some(vec!["unixsock-stream".to_string()]));
        config.listen.endpoints.set(
            ["unixsock-stream//tmp/sp_".to_string() + spaceport_name].iter().map(|s|s.parse().unwrap()).collect()
        ).unwrap();
        let session = zenoh::open(config).await.unwrap();
        Ok(session)
    }

    async fn open_radio(spaceport_id: &Ulid) -> io::Result<Session> {
        let mut config = zenoh::Config::default();
        config.set_id(ZenohId::try_from(spaceport_id.to_bytes().as_slice()).unwrap()).unwrap();
        config.set_mode(Some(WhatAmI::Peer));
        config.transport.link.set_protocols(Some(vec!["tcp".to_string(), "udp".to_string()]));
        config.listen.endpoints.set(
            [TCP_ENDPOINT, UDP_ENDPOINT].iter().map(|s|s.parse().unwrap()).collect()
        ).unwrap();
        let session = zenoh::open(config).await.unwrap();
        Ok(session)
    }

    async fn serve_api(port: u16) {
        let addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 
            port
        );
        let router = crate::api::configure();

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, router.into_make_service()).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber);

    let sp = Spaceport::open("test", 9999, &Settings::default()).await;
    let ext = sp.radio.zid();
    let int = sp.docks.info().zid().await;
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    sp.docks.put("key/expression", "value").await.unwrap();
    sp.radio.put("key/expression", "value").await.unwrap();
}


/// Banks hold assets and manage access.  They do not have communication
/// facilities of their own.  Relies on spaceport.
#[derive(Serialize, Deserialize)]
pub struct HoloBank {
    #[serde(skip_serializing, skip_deserializing)]
    path: String,
    id: Ulid,
    #[serde(skip_serializing, skip_deserializing)]
    vault: DbInstance,
    #[serde(skip_serializing, skip_deserializing)]
    cache: DbInstance,
    created: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    vault_capacity: u64,
    cache_size: u64,
}

impl Display for HoloBank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {}", self.id)
    }
}

impl HoloBank {
    /// Builds a new holobank from default values
    pub fn new(path: &str) -> HoloBank {
        let vault = DbInstance::new("rocksdb", path, "")
        .expect(format!("Failed to load rocksdb DbInstance from {}", path).as_str());
        let cache = DbInstance::new("mem", "", "")
        .expect("Failed to create in-memory DbInstance.");

        HoloBank::setup_vault(&vault);
        HoloBank::setup_cache(&cache);

        HoloBank {
            path: path.to_string(),
            id: Ulid::new(),
            vault,
            cache,
            created: Utc::now(),
            last_accessed: Utc::now(),
            vault_capacity: 8 * 1024 * 1024 * 1024,
            cache_size: 128 * 1024 * 1024,
        }
    }
    /// Builds a holobank from persistent storage
    pub fn from(path: &str) -> HoloBank {
        let (id, created, last_accessed, vault_capacity, cache_size) = HoloBank::load_config(&path);

        let vault = DbInstance::new("rocksdb", path, "")
        .expect(format!("Failed to load rocksdb DbInstance from {}", path).as_str());
        let cache = DbInstance::new("mem", "", "")
            .expect("Failed to create in-memory DbInstance.");
        
        HoloBank::verify_vault(&vault);
        HoloBank::setup_cache(&cache);

        HoloBank {
            path: path.to_string(),
            id,
            vault,
            cache,
            created,
            last_accessed,
            vault_capacity,
            cache_size,
        }
    }
    /// Gets the latest holoframe --subscribing if the data is not held
    /// locally.
    pub fn project<T>(id: Ulid) -> Result<T, AssetError> where
    T: Holographable {
        todo!()
    }
    /// Materialize a holographic asset.
    pub fn materialize<T, U>(hologram: T) -> Result<U, AssetError> where
    T: Holographable + Asset, U: Materializable + Asset {
        todo!()
    }
    /// Store a material asset in the holobank.
    /// Alias for deposit, digitize, or transfer.
    pub fn dematerialize<T>(asset: T, release: bool) -> Result<(), AssetError> where
    T: Materializable + Asset {
        todo!()
    }
    /// Load saved holobank parameter from config.json
    fn load_config(path: &str) -> (Ulid, DateTime<Utc>, DateTime<Utc>, u64, u64) {
        let filename = path.to_string() + "/config.json";
        match File::open(filename) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let config: HoloBank = serde_json::from_reader(reader).unwrap();
                return (config.id, config.created, config.last_accessed, config.vault_capacity, config.cache_size);
            }
            Err(e) => {
                panic!("Failed to load config from {}: {}", path, e);
            }
        }
    }
    /// Save holobank parameters to config.json
    fn save_config(bank: &HoloBank) {
        let filepath = bank.path.clone() + "/config.json";
        match File::create(&filepath) {
            Ok(file) => {
                let writer = BufWriter::new(file);
                serde_json::to_writer_pretty(writer, bank).unwrap();
            }
            Err(e) => {
                let json_string = serde_json::to_string_pretty(bank).unwrap();
                eprintln!("Error saving holobank config to file: {}", e);
                eprintln!("Save this to config.json at {}: \n{}", filepath, json_string);
            }
        }
    }
    /// Initializes vault stored relations
    fn setup_vault(db: &DbInstance) {
        db.run_default(schema::COMMMANDER_SCHEMA);
        db.run_default(schema::ASSET_SCHEMA);
        db.run_default(schema::NAME_SCHEMA);
        db.run_default(schema::CONTENT_SCHEMA);
        db.run_default(schema::OWNERSHIP_SCHEMA);
        db.run_default(schema::SNAPSHOT_SCHEMA);
        db.run_default(schema::HISTORY_SCHEMA);
        db.run_default(schema::CONNECTION_SCHEMA);
        db.run_default(schema::TAG_SCHEMA);
        db.run_default(schema::FLAG_SCHEMA);
        db.run_default(schema::COLLECTION_SCHEMA);
        db.run_default(schema::SPACEPORT_SCHEMA);
        db.run_default(schema::SYSTEM_SCHEMA);
        db.run_default(schema::STARMAP_SCHEMA);
    }
    /// Checks if Holobank belongs to spaceport
    fn verify_vault(db: &DbInstance) {
        todo!()
    }
    /// Initializes cache stored relations
    fn setup_cache(db: &DbInstance) {
        db.run_default(schema::CONTENT_SCHEMA);
        db.run_default(schema::HISTORY_SCHEMA);
    }
}
