pub mod json;

use alloc::{string::String, vec::Vec};

use crate::result::DataResult;

/// A [`CodecOps`] represents a way of converting Rust values into the target datatype and vice-versa.
/// [`CodecOps`] is the recommended way to do this when interacting with [`Codec`].
///
/// This trait is very low-level. This is intended as an interface for developers making their own datatypes that
/// can interact with [`Codec`]s. For a developer simply wishing to be able to serialize & deserialize data,
/// the [`Codec`] trait is recommended instead.
///
/// Since fixing data is a big part of the [`Codec`] API, [`Codec::decode`] accepts a mutable reference. This is because when trying to update the value,
/// it will try to optimize the updating and apply it to the top-level instead of creating new copies everywhere.
///
/// [`Codec`]: [`super::Codec`]
/// [`Codec::decode`]: [`super::Codec::decode`]
pub trait CodecOps<T>: Clone {
    /// Creates a new numeric value of type `T`. The range of the underlying number should be atleast of `f64` for maximum compatability.
    fn create_number(&self, value: &f64) -> T;
    /// Creates a new string value of type `T`.
    fn create_string(&self, value: &str) -> T;
    /// Creates a new boolean value of type `T`.
    fn create_boolean(&self, value: &bool) -> T;
    /// Creates a new list value of type `T`, containing other values of type `T`.
    fn create_list(&self, value: impl IntoIterator<Item = T>) -> T;
    /// Creates a new map type of type `T`. The iterator should be used to construct the map with the String as the key and the `T` as the value.
    fn create_map(&self, pairs: impl IntoIterator<Item = (String, T)>) -> T;
    /// Creates a new map type of type `T`. The value should have no associated fields or value. An empty map is a valid example of a representation.
    fn create_unit(&self) -> T;

    /// This converts a value of type `T` into a value of type `f64`.
    fn get_number(&self, value: &T) -> DataResult<f64>;
    /// This converts a value of type `T` into a value of type `String`.
    fn get_string(&self, value: &T) -> DataResult<String>;
    /// This converts a value of type `T` into a value of type `bool`.
    fn get_boolean(&self, value: &T) -> DataResult<bool>;
    /// This converts a value of type `T` into a view into a list's contents.
    fn get_list(&self, value: &mut T) -> DataResult<impl ListView<T>>;
    /// This converts a value of type `T` into a view into an map's contents.
    fn get_map(&self, value: &mut T) -> DataResult<impl MapView<T>>;
    /// This converts a value of type `T` into a unit value with no fields or associated values.
    fn get_unit(&self, value: &T) -> DataResult<()>;

    /// This purely exists for Optional Fields. The `Option` represents if a field is present,
    /// the `DataResult` represents the actual field data.
    fn create_map_special(
        &self,
        pairs: impl IntoIterator<Item = Option<DataResult<(String, T)>>>,
    ) -> DataResult<T> {
        let iter1 = pairs.into_iter().flatten().filter_map(|x| x.ok());

        Ok(self.create_map(iter1))
    }
}

/// Represents a lens into an map type from a [`CodecOps`]. Methods in this should be assumed to mutate - modifying the value using a [`MapView`]
/// will result in the underlying datastructures being mutated.
pub trait MapView<T> {
    /// Obtains a mutable reference to an underlying value. May return a DataError::KeyNotFoundInMap if the key is not present in the map.
    fn get(&mut self, name: &str) -> DataResult<&mut T>;
    /// Sets a key-value pair in the map to a certain value.
    fn set(&mut self, name: &str, value: T);
    /// Removes a certain key from the map, returning it's old value if the value was present. May return a DataError::KeyNotFoundInMap if the key
    /// was not present in the map before,
    fn remove(&mut self, key: &str) -> DataResult<T>;
    /// Obtains an owned and cloned list of keys that the map contained at the moment of calling.
    fn keys(&self) -> Vec<String>;
    /// Updates a value in the map to a new value if a value was already present under the key.
    fn update<F: FnOnce(&mut T)>(&mut self, name: &str, f: F) {
        if let Ok(v) = self.get(name) {
            f(v)
        }
    }
}

/// Represents a lens into an list type from a [`CodecOps`]. Methods in this should be assumed to mutate - modifying the value using a [`ListView`]
/// will result in the underlying datastructures being mutated.
pub trait ListView<T> {
    /// Appends a new value to the list. This may allocate.
    fn append(&mut self, value: T);
    /// Gets a mutable reference to a value at an index inside of a list. May return a DataError::ListIndexOutOfBounds if the index is out of bounds.
    /// This is up to the implementor of this method to check.
    fn get(&mut self, index: usize) -> DataResult<&mut T>;
    /// This consumes the value inside of the ListView and turns it into an iterator. This method may change in the near future.
    fn into_iter(self) -> impl Iterator<Item = T>;
}
