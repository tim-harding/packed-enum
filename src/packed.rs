use crate::EnumInfo;
use std::marker::PhantomData;

pub struct Packed<T: EnumInfo> {
    buckets: Vec<Vec<u8>>,
    marker: PhantomData<T>,
}

impl<T: EnumInfo> Packed<T>
where
    [(); T::VARIANT_COUNT]:,
    [(); T::SIZES_COUNT]:,
{
}

const fn variant_sizes_unique<const N: usize, const U: usize, T: EnumInfo>() -> [usize; U] {
    let mut out = [0; U];
    let deduped = variant_sizes_deduped::<N, T>();
    let mut i = 0;
    let mut j = 0;
    while i < N {
        if deduped[i] > 0 {
            out[j] = deduped[i];
            j += 1;
        }
        i += 1;
    }
    out
}

const fn variant_sizes_count<const N: usize, T: EnumInfo>() -> usize {
    let mut count = 0;
    let unique_variant_sizes = variant_sizes_deduped::<N, T>();
    let mut i = 0;
    while i < N {
        if unique_variant_sizes[i] > 0 {
            count += 1;
        }
        i += 1;
    }
    count
}

const fn variant_sizes_deduped<const N: usize, T: EnumInfo>() -> [usize; N] {
    let mut variant_sizes = variant_sizes::<N, T>();
    let mut i = 0;
    while i < N {
        let mut j = i + 1;
        while j < N {
            if variant_sizes[j] == variant_sizes[i] {
                variant_sizes[j] = 0;
            }
            j += 1;
        }
        i += 1;
    }
    variant_sizes
}

const fn variant_sizes<const N: usize, T: EnumInfo>() -> [usize; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        let variant = &T::VARIANTS[i];
        let mut size = 0;

        let mut j = 0;
        while j < variant.len() {
            let field = &variant[j];
            size += field.size;
            j += 1;
        }

        out[i] = size;
        i += 1;
    }
    out
}

const fn variant_count<T: EnumInfo>() -> usize {
    T::VARIANTS.len()
}
