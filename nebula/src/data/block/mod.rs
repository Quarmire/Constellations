pub mod text;

/// A block is a unit of stateless data.
// pub struct Block<T> {
//     ulid: String,
//     name: Option<String>,
//     content: T,
//     holder: String,
//     context: String,
//     metadata: String,
// }

/// A block implies building and connecting individual pieces to form
/// a larger whole.
pub trait Block {}




// Storage is important!  Design decisions here are heavy.
// We want fast retrieval and storage but also the ability to query
// the data where it is stored.
// Upon request for modification, the data is pulled out to construct
// a block object of the specified type.
// When the modification is completed, the object is broken down and
// stored in the appropriate tables.
// Serialization is for transferring between two processes or system.

// Blocks are sent across spaceports by id and name.
// This is done automatically by the banks.

// Each spaceport lists the crewed ships docked in port as well as crew and agents that reside there and the facilities of the spaceport.

// Banks are networked meaning they can request resource transfers from one another
// Banks, by default, do not synchronize assets unless it is added to a collection
// A collection is a list of assets that are related in some manner
// A collection is also a schema which defines which spaceports will track the collection.

// Data not in a collection will instead be stored in a local collection (buffer).

// Banks store blocks, blueprints, and manage assemblies.
// Banks form networks to synchronize their assets.
// Assemblies are constructed from blueprints (page, book, bookshelf, room, house)