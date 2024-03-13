use crate::{byte_vec::ByteVec, EnumInfo};
use std::marker::PhantomData;

pub struct Packed<T>
where
    T: EnumInfo,
    [(); T::SIZES_COUNT]:,
    [(); T::SIZES.len()]:,
{
    entries: Vec<Entry>,
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

struct Entry {
    variant: usize,
    index_in_bucket: usize,
}
