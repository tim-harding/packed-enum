use crate::{byte_vec::ByteVec, AsIndex, EnumInfo};
use std::marker::PhantomData;

// TODO:
// - Replace generated structs with offset_of!
// - Copy from enum start to last byte of variant to the storage

pub struct Packed<T>
where
    T: EnumInfo,
{
    // TODO: Memory compaction of entries
    entries: Vec<Entry<T>>,
    // TODO: Use array instead when generic_const_exprs is stable
    buckets: Vec<ByteVec>,
    marker: PhantomData<T>,
}

impl<T> Packed<T>
where
    T: EnumInfo,
{
    /// Creates a new, empty collection.
    pub fn new() -> Self {
        let buckets: Vec<_> = std::iter::repeat_with(ByteVec::new)
            .take(T::SIZES.len())
            .collect();
        Self {
            // TODO: Make new const when from_fn is const
            buckets,
            entries: vec![],
            marker: PhantomData,
        }
    }

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

impl<T> Default for Packed<T>
where
    T: EnumInfo,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Packed<T>
where
    T: EnumInfo,
{
    fn drop(&mut self) {
        for (bucket, size) in self.buckets.iter_mut().zip(T::SIZES) {
            unsafe {
                bucket.dealloc(*size);
            }
        }
    }
}

struct Entry<T>
where
    T: EnumInfo,
{
    variant: T::Variant,
    index: usize,
}
