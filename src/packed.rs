use crate::{variant_size, EnumInfo};
use std::marker::PhantomData;

pub struct Packed<T: EnumInfo> {
    buckets: Vec<Vec<u8>>,
    marker: PhantomData<T>,
}

impl<T: EnumInfo> Packed<T>
where
    [(); T::SIZES_COUNT]:,
{
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
}
