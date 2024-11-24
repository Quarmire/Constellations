use std::path::Path;

use anyhow::Result;
use cozo::DbInstance;

mod schema;

#[derive(Clone)]
struct Holobank {
    persistent: DbInstance,
    cache: DbInstance,
}

impl Holobank {
    fn load(path: &Path) -> Result<Holobank, cozo::Error> {
        let persistent = if path.exists() {
            DbInstance::new("rocksdb", path, "")?
        }
        else {
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
        Ok(db)
    }
    
    fn setup_cache() -> Result<DbInstance, cozo::Error> {
        let db = DbInstance::new("mem", "", "")?;
        db.run_default(schema::CONTENT_SCHEMA);
        db.run_default(schema::HISTORY_SCHEMA);
        Ok(db)
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