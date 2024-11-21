use crate::{
    byte_vec::{ByteVec, WrapVec},
    Packable, Variant,
};
use std::marker::PhantomData;

macro_rules! bucket {
    ($s:ident, $v:ident) => {{
        let (size, align) = $v.size_align();
        let bucket = &mut $s.buckets[$v.as_index()];
        unsafe { WrapVec::new(bucket, size, align) }
    }};
}

pub struct Pack<T: Packable> {
    // TODO: Memory compaction of entries
    entries: Vec<Entry<T>>,
    // TODO: Use array instead when generic_const_exprs is stable
    buckets: Vec<ByteVec>,
    marker: PhantomData<T>,
}

impl<T: Packable> Pack<T> {
    /// Creates a new, empty collection.
    pub fn new() -> Self {
        let buckets: Vec<_> = std::iter::repeat_with(ByteVec::new)
            .take(T::VARIANT_COUNT)
            .collect();
        Self {
            buckets,
            entries: vec![],
            marker: PhantomData,
        }
    }

    /// Returns the number of elements in the slice
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    pub fn push(&mut self, element: T) {
        let variant = element.variant();
        let mut bucket = bucket!(self, variant);

        let index = bucket.len();
        bucket.maybe_grow_by(1);
        bucket.set_len(index + 1);

        let dst = bucket.get_mut(index);
        unsafe { element.write(dst) };

        self.entries.push(Entry { variant, index });
    }

    pub fn pop(&mut self) -> Option<T> {
        self.entries.pop().map(|entry| {
            let Entry { variant, index } = entry;
            let mut bucket = bucket!(self, variant);
            let src = bucket.get(index);
            bucket.set_len(index);
            unsafe { T::read(variant, src) }
        })
    }
}

impl<T: Packable> Drop for Pack<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
        for variant in <T::Variant as Variant>::all() {
            bucket!(self, variant).dealloc();
        }
    }
}

impl<T: Packable> Default for Pack<T> {
    fn default() -> Self {
        Self::new()
    }
}

struct Entry<T: Packable> {
    variant: T::Variant,
    index: usize,
}
