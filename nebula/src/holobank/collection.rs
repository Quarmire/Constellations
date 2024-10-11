use std::{collections::HashMap, io};

use ulid::Ulid;

use super::asset::AssetType;


/// A list of asset ids that can be shared between spaceports.
struct Collection {
    id: Ulid,
    name: String,
    assets: HashMap<Ulid, AssetType>
}

impl Collection {
    /// Create a new collection
    fn new() -> Collection {
        todo!()
    }
    /// Fork an exisiting collection
    fn fork() -> Collection {
        todo!()
    }
    /// Load a collection from file
    fn from_file() -> io::Result<Collection> {
        todo!()
    }
    /// Load a collection from database
    fn from_bank() -> io::Result<Collection> {
        todo!()
    }

    fn add_asset(id: Ulid) {

    }

    fn remove_asset(id: Ulid) {

    }

    fn destroy() {

    }
}