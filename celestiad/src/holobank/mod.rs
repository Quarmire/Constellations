use std::{collections::BTreeMap, path::Path};

use anyhow::Result;
use cozo::{DataValue, DbInstance, ScriptMutability, UlidWrapper};
use tracing::{debug, info};
use ulid::Ulid;

mod schema;

#[derive(Clone)]
pub struct Holobank {
    persistent: DbInstance,
    cache: DbInstance,
}

impl Holobank {
    pub fn load(path: &Path) -> Result<Holobank, cozo::Error> {
        debug!("Attempting to load holobank from {:?}", path);
        let persistent = if path.exists() {
            info!("Loading holobank from {:?}", path);
            DbInstance::new("rocksdb", path, "")?
        }
        else {
            info!("Holobank does not exist at {:?}. Creating holobank", path);
            Holobank::setup_persistent(path)?
        };

        let cache = Holobank::setup_cache()?;

        Ok(Holobank {
            persistent,
            cache
        })
    }

    fn setup_persistent(path: &Path) -> Result<DbInstance, cozo::Error> {
        let db = DbInstance::new("rocksdb", path, "")?;
        db.run_default(schema::COMMMANDER_SCHEMA)?;
        db.run_default(schema::ASSET_SCHEMA)?;
        db.run_default(schema::NAME_SCHEMA)?;
        db.run_default(schema::CONTENT_SCHEMA)?;
        db.run_default(schema::OWNERSHIP_SCHEMA)?;
        db.run_default(schema::SNAPSHOT_SCHEMA)?;
        db.run_default(schema::HISTORY_SCHEMA)?;
        db.run_default(schema::CONNECTION_SCHEMA)?;
        db.run_default(schema::TAG_SCHEMA)?;
        db.run_default(schema::FLAG_SCHEMA)?;
        db.run_default(schema::COLLECTION_SCHEMA)?;
        db.run_default(schema::SPACEPORT_SCHEMA)?;
        db.run_default(schema::SYSTEM_SCHEMA)?;
        db.run_default(schema::STARMAP_SCHEMA)?;
        Ok(db)
    }
    
    fn setup_cache() -> Result<DbInstance, cozo::Error> {
        let db = DbInstance::new("mem", "", "")?;
        db.run_default(schema::CONTENT_SCHEMA)?;
        db.run_default(schema::HISTORY_SCHEMA)?;
        Ok(db)
    }

    pub fn get_text_block_ids(&self) -> String {
        let mut parameters = BTreeMap::new();
        parameters.insert("type".to_string(), DataValue::Str("text".into()));
        match self.persistent.run_script(
            "?[a] := *content{asset_id: a, content_type: $type}",
            parameters.clone(),
            ScriptMutability::Mutable
        ) {
            Ok(rows) => {
                let agg: Vec<String> = rows.into_iter().map(|x| {x[0].get_ulid().unwrap().to_string()}).collect();
                agg.join(",").to_string()
            }
            Err(e) => {
                debug!("Could not get text block ids: {}", e);
                "".to_string()
            }
        }
    }

    pub fn get_text_block_content(&self, id: Ulid) -> Option<String> {
        let mut parameters = BTreeMap::new();
        parameters.insert("id".to_string(), DataValue::Ulid(UlidWrapper(id)));
        parameters.insert("type".to_string(), DataValue::Str("text".into()));
        match self.persistent.run_script(
            "?[a] := *content{asset_id: $id, content_type: $type, content: a}",
            parameters.clone(),
            ScriptMutability::Mutable
        ) {
            Ok(rows) => {
                if rows.rows.len() < 1 { // block does not exist
                    return None;
                }
                else {
                    let content = rows.rows[0][0].get_str().unwrap().to_string();
                    return Some(content);
                }
            }
            Err(e) => {
                debug!("Could not lookup text block id: {}; {}", id, e);
                return None;
            }
        }
    }

    pub fn set_text_block_content(&self, id: Ulid, content: String) -> Result<()> {
        let mut parameters = BTreeMap::new();
        parameters.insert("id".to_string(), DataValue::Ulid(UlidWrapper(id)));
        parameters.insert("type".to_string(), DataValue::Str("text".into()));
        parameters.insert("content".to_string(), DataValue::Str(content.into()));
        parameters.insert("time".to_string(), DataValue::Num(cozo::Num::Int(0)));
        match self.persistent.run_script(
            "?[asset_id, content_type, content, time_attached] <- [[$id, $type, $content, $time]] :put content {asset_id, content_type, content, time_attached}",
            parameters.clone(),
            ScriptMutability::Mutable
        ) {
            Ok(_) => {
                info!("Added text block with ID: {}", id.to_string());
            }
            Err(e) => {
                debug!("Could not add content to holobank: {}", e);
            }
        }
        Ok(())
    }
}



// Gets the latest holoframe --subscribing if the data is not held
    // locally.
    // pub fn project<T>(id: Ulid) -> Result<T, AssetError> where
    // T: Holographable {
    //     todo!()
    // }
    // /// Materialize a holographic asset.
    // pub fn materialize<T, U>(hologram: T) -> Result<U, AssetError> where
    // T: Holographable + Asset, U: Materializable + Asset {
    //     todo!()
    // }
    // /// Store a material asset in the holobank.
    // /// Alias for deposit, digitize, or transfer.
    // pub fn dematerialize<T>(asset: T, release: bool) -> Result<(), AssetError> where
    // T: Materializable + Asset {
    //     todo!()
    // }