use ulid::Ulid;

use crate::data::Asset;

mod collection;

// Banks hold and manage assets.
pub struct Bank {
    
}

impl Bank {
    pub fn new() -> Bank {
        Bank {  }
    }

    // Local bank operations
    /// Storage (cozodb) -> Memory
    pub fn withdraw<T>(id: Ulid) -> T
    where T: Asset {
        // cozodb_get_and_build(id) -> T
    }
    /// Memory -> Storage (cozodb)
    pub fn deposit<T>(asset: T)
    where T: Asset {

    }

    // Bank network operations
    /// Request an asset from an in-network bank
    pub fn obtain<T>(id: Ulid) -> T
    where T: Asset {

    }
    /// Send an asset to another bank
    pub fn transfer<T>(asset: T)
    where T: Asset {

    }
}