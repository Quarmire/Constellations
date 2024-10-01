use std::collections::HashMap;

/// A block is a unit of data.
pub struct Block<T> {
    name: Option<String>,
    content: T,
    holder: String,
}