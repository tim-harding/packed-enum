use crate::{byte_vec::ByteVec, AsIndex, EnumInfo};
use std::marker::PhantomData;

pub struct Pack<T>
where
    T: EnumInfo,
{
    // TODO: Memory compaction of entries
    entries: Vec<Entry<T>>,
    // TODO: Use array instead when generic_const_exprs is stable
    buckets: Vec<ByteVec>,
    marker: PhantomData<T>,
}

impl<T> Pack<T>
where
    T: EnumInfo,
{
    /// Creates a new, empty collection.
    pub fn new() -> Self {
        let buckets: Vec<_> = std::iter::repeat_with(ByteVec::new)
            .take(T::SIZES.len())
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
        let variant_index = variant.as_index();
        let size = T::SIZES[variant_index];
        let bucket = &mut self.buckets[variant_index];
        let index = bucket.len();
        unsafe {
            bucket.grow(size);
        }
        let dst = unsafe { bucket.get_mut(index, size) };
        element.write(dst);
        self.entries.push(Entry { variant, index })
    }

    pub fn pop(&mut self) -> Option<T> {
        self.entries.pop().map(|entry| {
            let Entry { variant, index } = entry;
            let variant_index = variant.as_index();
            let size = T::SIZES[variant_index];
            let bucket = &mut self.buckets[variant_index];
            let src = unsafe { bucket.get(index, size) };
            unsafe {
                bucket.swap_remove(index, size);
            }
            T::read(variant, src)
        })
    }
}

impl<T> Default for Pack<T>
where
    T: EnumInfo,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Pack<T>
where
    T: EnumInfo,
{
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

struct Entry<T>
where
    T: EnumInfo,
{
    variant: T::Variant,
    index: usize,
}
