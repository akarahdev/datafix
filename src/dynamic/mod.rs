use alloc::string::String;

use crate::{
    result::DataResult,
    serialization::{CodecOps, ListView, MapView},
};

pub struct Dynamic<'a, T, O: CodecOps<T>> {
    ops: O,
    value: &'a mut T,
}

impl<'a, T, O: CodecOps<T>> Dynamic<'a, T, O> {
    pub fn new(ops: O, value: &mut T) -> Dynamic<T, O> {
        Dynamic { ops, value }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_inner(self) -> &'a mut T {
        self.value
    }

    pub fn ops(&self) -> &O {
        &self.ops
    }

    pub fn mutate<F: FnOnce(&mut T)>(&mut self, f: F) {
        f(&mut self.value);
    }

    pub fn as_number(&self) -> DataResult<f64> {
        self.ops.get_number(&self.value)
    }

    pub fn as_string(&self) -> DataResult<String> {
        self.ops.get_string(&self.value)
    }

    pub fn as_boolean(&self) -> DataResult<bool> {
        self.ops.get_boolean(&self.value)
    }

    pub fn as_unit(&self) -> DataResult<()> {
        self.ops.get_unit(&self.value)
    }

    pub fn as_map(&mut self) -> DataResult<impl MapView<T>> {
        self.ops.get_map(self.value)
    }

    pub fn as_list(&mut self) -> DataResult<impl ListView<T>> {
        self.ops.get_list(self.value)
    }
}
