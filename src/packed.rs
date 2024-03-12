use crate::{byte_vec::ByteVec, variant_size, EnumInfo};
use std::{marker::PhantomData, mem::ManuallyDrop, ops::Deref};

pub struct Packed<T>
where
    T: EnumInfo,
    [(); T::SIZES_COUNT]:,
    [(); T::VARIANT_COUNT]:,
{
    entries: Vec<Entry>,
    buckets: [ByteVec; T::SIZES_COUNT],
    marker: PhantomData<T>,
}

impl<T> Packed<T>
where
    T: EnumInfo,
    [(); T::SIZES_COUNT]:,
    [(); T::VARIANT_COUNT]:,
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

    pub fn push(&mut self, element: T) {
        let element = ManuallyDrop::new(element);
        let variant = element.variant_index();
        let bucket_index = Self::BUCKET[variant];
        if let Some(bucket_index) = bucket_index {
            let size = Self::SIZES[variant];
            let bound_hi = Self::COPY_END_BOUND[variant];
            let bound_lo = bound_hi - size;
            let ptr = std::ptr::from_ref(element.deref()).cast::<u8>();
            let ptr = unsafe { ptr.add(bound_lo) };
            let bucket = &mut self.buckets[bucket_index];
            let index_in_bucket = bucket.len();
            unsafe { bucket.push(ptr, size) };
            self.entries.push(Entry {
                variant,
                index_in_bucket,
            })
        } else {
            self.entries.push(Entry {
                variant,
                index_in_bucket: 0,
            });
        }
    }

    pub const SIZES: [usize; T::SIZES_COUNT] = {
        let mut out = [0usize; T::SIZES_COUNT];
        let variants = T::VARIANTS;
        let mut prev_largest = 0;

        let mut i = 0;
        while i < T::SIZES_COUNT {
            let mut next_largest = usize::MAX;

            let mut j = 0;
            while j < variants.len() {
                let variant = variants[j];
                let size = variant_size(variant);
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

    pub const COPY_END_BOUND: [usize; T::VARIANT_COUNT] = {
        let mut out = [0; T::VARIANT_COUNT];
        let variants = T::VARIANTS;

        let mut i = 0;
        while i < T::VARIANT_COUNT {
            let variant = variants[i];

            if variant.len() > 0 {
                let mut max = 0;

                let mut j = 0;
                while j < variant.len() {
                    let field = &variant[j];

                    let hi = field.offset + field.size;
                    max = if max > hi { max } else { hi };

                    j += 1;
                }

                out[i] = max;
            }

            i += 1;
        }
        out
    };

    pub const BUCKET: [Option<usize>; T::VARIANT_COUNT] = {
        let mut out = [None; T::VARIANT_COUNT];
        let variants = T::VARIANTS;

        let mut i = 0;
        while i < T::VARIANT_COUNT {
            let size = variant_size(variants[i]);

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
    [(); T::VARIANT_COUNT]:,
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
