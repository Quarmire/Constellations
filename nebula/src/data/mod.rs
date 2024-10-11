pub mod block;
mod blueprint;
mod assembly;
mod file;

use ulid::Ulid;

/// 
pub trait Asset {
    /// Get asset ID
    fn id(&self) -> Ulid;
    /// Set asset name
    fn name(&self);
    /// Add a tag to the asset
    fn tag(&self);
    /// Remove a tag from the asset
    fn untag(&self);
    /// Add a flag to the asset
    fn flag(&self);
    /// Remove a flag from the asset
    fn unflag(&self);
    /// Flag an asset for dematerialization.
    fn release(&self);
    /// Uploads asset data to holobank.  Not dematerialize.
    fn upload();
    /// Downloads asset data from holobank.  Not materialize.
    fn download() -> Self;
    /// Create a new asset from another (crop, slice, paste, etc)
    fn derive() -> Self;
    /// Create an alternative version of an asset
    fn fork() -> Self;
}

pub enum AssetType {
    Block,
    Blueprint,
    Assembly,
    File,
}

pub struct AssetState {
    held: bool,
    here: bool,
    holo: bool,
    live: bool,
}

pub struct AssetFlags {
    expires: bool,
    junk: bool,
    draft: bool,
}

/// Implies readability.
/// Any number of holograms can exist at any given time.
/// Holograms are the spitting image of an asset and update
/// to match the asset whenever possible.
pub trait Holographable {
    /// Stores the latest holoframe in the holobank.  Generally called when bank
    /// subscriber receives a change.  Only updates the asset in the holobank.
    /// Holograms outside the bank must subscribe to know when an asset is updated.
    fn update();
}

/// Implies writability.
/// Only one material asset is allowed in existence at any given time.
pub trait Materializable {
    /// Scan and upload asset current state to holobank.  Generally called upon
    /// remote query.
    fn scan();
}

pub struct Hologram<T: Asset + Holographable + Materializable> {
    asset: T,
}

// By default, an asset is scanned to the holobank after every context change.
// To support real-time updates, the scanner can be set to polling or event-based.
// The asset is copied to the in-memory database and updates the RocksDB database
// on context-change.

pub enum AssetError {
    Busy,
    UndefinedError
}

pub trait Import {
    /// Create a block from resources outside the spaceport i.e.,
    /// documents
    fn import() -> Self;
}

pub trait Export {
     /// Send a block to consumers outside the spaceport
     fn export(&self);
}