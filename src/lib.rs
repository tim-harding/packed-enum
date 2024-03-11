pub use packed_enum_derive::EnumInfo;

pub trait EnumInfo {
    const VARIANTS: &'static [&'static [VariantField]];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VariantField {
    pub size: usize,
    pub align: usize,
    pub offset: usize,
}

#[allow(unused)]
const fn dedup<const N: usize>(sizes: [usize; N]) -> [Option<usize>; N] {
    let mut unique = [None; N];
    let mut i = 0;
    let mut unique_index = 0;
    while i < sizes.len() {
        let mut contains = false;
        let mut j = 0;
        while j < unique.len() {
            let Some(u) = unique[j] else {
                break;
            };
            if u == sizes[i] {
                contains = true;
                break;
            }
            j += 1;
        }

        if !contains {
            unique[unique_index] = Some(sizes[i]);
            unique_index += 1;
        }

        i += 1;
    }
    unique
}

#[allow(unused)]
const fn count_some<const N: usize>(sizes: [Option<usize>; N]) -> usize {
    let mut i = 0;
    while i < N {
        if sizes[i].is_some() {
            i += 1;
        } else {
            break;
        }
    }
    i
}

#[allow(unused)]
const fn first_n_sorted<const I: usize, const O: usize>(
    mut array: [Option<usize>; I],
) -> [usize; O] {
    let mut out = [0; O];
    let mut i = 0;
    while i < O {
        let mut min = usize::MAX;
        let mut min_j = 0;
        let mut j = 0;
        while j < O {
            if let Some(n) = array[j] {
                if n < min {
                    min = n;
                    min_j = j;
                }
            }
            j += 1;
        }
        let Some(n) = array[min_j] else {
            panic!();
        };
        array[min_j] = None;
        out[i] = n;
        i += 1;
    }
    out
}
