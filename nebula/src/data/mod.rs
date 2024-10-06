pub mod block;
mod blueprint;
mod assembly;

use assembly::Assembly;
use block::Block;
use blueprint::Blueprint;
use ulid::Ulid;

/// Sets of functions a type must implement to be managed by a bank.
pub trait Asset {
    fn id() -> Ulid {}
    fn name() -> Option<String> {}

    // Local bank operations
    /// Storage (cozodb) -> Memory
    fn withdraw<T>(id: Ulid) -> T 
    where T: Block + Blueprint + Assembly {
        todo!("Implement the withdraw function to return a value of the expected type T")
    }
    /// Memory -> Storage (cozodb)
    fn deposit<T>(asset: T) {}
    fn update(id: Ulid) {}

    // Bank network operations
    /// Request an asset from an in-network bank
    fn obtain(id: Ulid) {}
    /// Send an asset to another bank
    fn transfer<T>(asset: T) {}
}

pub trait Realtime {
    fn sync() {}
}

trait Search {
    
}

trait Vector {
    fn vectorize() {}
}

trait View {

}
