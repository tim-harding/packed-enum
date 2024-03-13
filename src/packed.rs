use crate::{byte_vec::ByteVec, AsIndex, EnumInfo};
use std::marker::PhantomData;

pub struct Packed<T>
where
    T: EnumInfo,
    [(); T::SIZES_COUNT]:,
    [(); T::SIZES.len()]:,
{
    // TODO: Memory compaction of entries
    entries: Vec<Entry<T>>,
    buckets: [ByteVec; T::SIZES_COUNT],
    marker: PhantomData<T>,
}

impl<T> Packed<T>
where
    T: EnumInfo,
    [(); T::SIZES_COUNT]:,
    [(); T::SIZES.len()]:,
{
    /// Creates a new, empty collection.
    pub fn new() -> Self {
        Self {
            // TODO: Make new const when from_fn is const
            buckets: std::array::from_fn(|_| ByteVec::new()),
            entries: vec![],
            marker: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    pub fn push(&mut self, element: T) {
        let variant = element.variant();
        let variant_index = variant.as_index();
        if let Some(bucket) = Self::BUCKET[variant_index] {
            let size = T::SIZES[variant_index];
            let bucket = &mut self.buckets[bucket];
            let index = bucket.len();
            unsafe {
                bucket.grow(size);
            }
            let dst = unsafe { bucket.get_mut(index, size) };
            element.write(dst);
            self.entries.push(Entry { variant, index })
        } else {
            self.entries.push(Entry { variant, index: 0 })
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.entries.pop().map(|entry| {
            let Entry { variant, index } = entry;
            let variant_index = variant.as_index();
            if let Some(bucket) = Self::BUCKET[variant_index] {
                let size = T::SIZES[variant_index];
                let bucket = &mut self.buckets[bucket];
                let src = unsafe { bucket.get(index, size) };
                unsafe {
                    bucket.swap_remove(index, size);
                }
                T::read(variant, src)
            } else {
                // Null is okay here because we don't read the pointer for zero-sized variants
                T::read(variant, std::ptr::null())
            }
        })
    }

    pub const SIZES: [usize; T::SIZES_COUNT] = {
        let mut out = [0usize; T::SIZES_COUNT];
        let sizes = T::SIZES;
        let mut prev_largest = 0;

        let mut i = 0;
        while i < T::SIZES_COUNT {
            let mut next_largest = usize::MAX;

            let mut j = 0;
            while j < sizes.len() {
                let size = sizes[j];
                if size > prev_largest && size < next_largest {
                    next_largest = size;
                }
                j += 1;
            }

            prev_largest = next_largest;
            out[i] = next_largest;
            i += 1;
        }
        out
    };

    pub const BUCKET: [Option<usize>; T::SIZES.len()] = {
        let mut out = [None; T::SIZES.len()];
        let sizes = T::SIZES;

        let mut i = 0;
        while i < T::SIZES.len() {
            let size = sizes[i];

            let mut j = 0;
            while j < T::SIZES_COUNT {
                if Self::SIZES[j] == size {
                    out[i] = Some(j);
                    break;
                }
                j += 1;
            }
            i += 1;
        }

        out
    };
}

impl<T> Drop for Packed<T>
where
    T: EnumInfo,
    [(); T::SIZES_COUNT]:,
    [(); T::SIZES.len()]:,
{
    fn drop(&mut self) {
        for (bucket, size) in self.buckets.iter_mut().zip(Self::SIZES) {
            unsafe {
                bucket.dealloc(size);
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
