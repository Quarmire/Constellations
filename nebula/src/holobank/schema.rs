/// Stores holobank information
pub const HOLOBANK_SCHEMA: &str = "
    :create holobank {
        id: Ulid,
        =>
        spaceport_id: Ulid,
        time_created: Int,
    }
";

/// Users
pub const COMMMANDER_SCHEMA: &str = "
    :create commander {
        id: Ulid,
        username: String,
        birthdate: String,
    }
";

/// Asset identification
pub const ASSET_SCHEMA: &str = "
    :create asset {
        asset_id: Ulid,
        name: String? default null,
        =>
        asset_type: String,
        time_registered: Int,
    }
";

/// Associate one or more names for all ids except assets.
/// Name associations are tied to the commander.
pub const NAME_SCHEMA: &str = "
    :create name {
        id: Ulid,
        name: String,
        =>
        by: Ulid,
        time_named: Int,
    }
";

/// Asset content.  Also used in in-memory database.
pub const CONTENT_SCHEMA: &str = "
    :create content {
        asset_id: Ulid,
        =>
        content_type: String,
        content: Any,
        time_attached: Int,
    }
";

// Ownership for all things, i.e., assets and spaceports
pub const OWNERSHIP_SCHEMA: &str = "
    :create owner {
        id: Ulid,
        commander_id: Ulid,
        time: Validity,
    }
";

/// Stores a copy of an asset frozen in time
pub const SNAPSHOT_SCHEMA: &str = "
    :create snapshot {
        asset_id: Ulid,
        time: Validity,
        =>
        latest: Any,
    }
";

/// Stores edits of content.  In-memory stores individual edits while
/// persistent stores a series of edits.  The time is an index in
/// memory while it is a timestamp in persistent.
pub const HISTORY_SCHEMA: &str = "
    :create history {
        asset_id: Ulid,
        time: Validity,
        =>
        edit: Any,
    }
";

/// A directed edge between nodes which can be anything with an ULID identifier.
/// Type describes the connection; derivation, fork, IP, etc.
pub const CONNECTION_SCHEMA: &str = "
    :create connection {
        src: Ulid,
        dest: Ulid,
        type: String,
        =>
        time_created: Int,
    }
";

/// Assets can have multiple tags and tags can tag describe multiple assets.
pub const TAG_SCHEMA: &str = "
    :create tags {
        asset_id: Ulid,
        tag: String,
        =>
        time_attached: Int,
    }
";

/// Flags signal actions/states for an asset.
pub const FLAG_SCHEMA: &str = "
    :create flags {
        asset_id: Ulid,
        flag: String,
        =>
        time_attached: Int,
    }
";

/// Records asset materializations by commander at some spaceport and at some time.
/// Also records which asset was last materialized prior to materializing this one.
pub const ACCESS_SCHEMA: &str = "
    :create access {
        asset: Ulid,
        time: Validity,
        =>
        materialized_by: Ulid,
        materialized_at: Ulid,
        coming_from: Ulid,
    }
";

/// Depending on the asset type, the relation between asset and collection is different.
/// A block is a part of the collection.
/// A blueprint is associated with the blocks of the collection.
/// An assembly is associated with a blueprint in the collection.
/// Collection ULID encodes the time collection is created.
pub const COLLECTION_SCHEMA: &str = "
    :create collection {
        asset_id: Ulid,
        collection_id: Ulid,
        =>
        time_added: Int,
    }
";

/// Spaceports can be hosted on planets or ships.
pub const SPACEPORT_SCHEMA: &str = "
    :create spaceport {
        spaceport_id: Ulid,
        host_name: String,
        host_type: String,
        =>
        time_created: Int,
        collections: [Ulid]?,
        last_visited_system: Ulid,
    }
";

/// A system encompasses the entirety of a LAN or P2P network.
/// The info attribute are landmarks that identifies a system.
/// The protocol attribute enables communication in the system.
pub const SYSTEM_SCHEMA: &str = "
    :create system {
        system_id: Ulid,
        is_star_system: Bool,
        =>
        info: Json,
        protocol: Json,
    }
";

/// Mapping known spaceports to known systems.
pub const STARMAP_SCHEMA: &str = "
    :create starmap {
        system_id: Ulid,
        spaceport_id: Ulid,
    }
";