use std::{fmt::Display, fs::File, io::{BufReader, BufWriter}};

use cozo::DbInstance;
use serde::{Deserialize, Serialize};
use ulid::Ulid;
use chrono::{DateTime, Utc};

use crate::data::{Asset, AssetError, Holographable, Materializable};

mod collection;

/// Banks hold and manage assets.  They do not have communication
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
        .expect("Failed to create in-memory DbInstance.#[tokio::main]");

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

    }
    /// Checks if Holobank ID matches DbInstance ID
    fn verify_vault(db: &DbInstance) {

    }
    /// Initializes cache stored relations
    fn setup_cache(db: &DbInstance) {

    }
}