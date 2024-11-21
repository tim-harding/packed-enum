use crate::{
    byte_vec::{ByteVec, WrapVec},
    Packable, Variant,
};
use std::marker::PhantomData;

pub struct Pack<T>
where
    T: Packable,
{
    // TODO: Memory compaction of entries
    entries: Vec<Entry<T>>,
    // TODO: Use array instead when generic_const_exprs is stable
    buckets: Vec<ByteVec>,
    marker: PhantomData<T>,
}

impl<T> Pack<T>
where
    T: Packable,
{
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
        let (size, align) = variant.size_align();

        let bucket = &mut self.buckets[variant.as_index()];
        let mut bucket = unsafe { WrapVec::new(bucket, size, align) };

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
            let (size, align) = variant.size_align();

            let bucket = &mut self.buckets[variant.as_index()];
            let bucket = unsafe { WrapVec::new(bucket, size, align) };

            let src = bucket.get(index);
            unsafe { T::read(variant, src) }
        })
    }
}

impl<T: Packable> Default for Pack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Packable> Drop for Pack<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

struct Entry<T: Packable> {
    variant: T::Variant,
    index: usize,
}
