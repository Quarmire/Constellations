mod collection;

use super::super::data::Asset;

// Banks hold and manage assets.
pub struct Bank<T: Asset<T>> {
    datatype: T
}
