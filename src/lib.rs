#![feature(offset_of_nested)]
#![feature(offset_of_enum)]

pub use packed_enum_derive::Packed;
use std::mem::{align_of, offset_of, size_of};

enum Test {
    Things { hello: u16, world: u8, stuff: u64 },
    Stuff { what: u8, ever: u16 },
    This(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Field {
    size: usize,
    align: usize,
    offset: usize,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let things = vec![
            Field {
                offset: offset_of!(Test, Things.hello),
                size: size_of::<u16>(),
                align: align_of::<u16>(),
            },
            Field {
                offset: offset_of!(Test, Things.world),
                size: size_of::<u8>(),
                align: align_of::<u8>(),
            },
            Field {
                offset: offset_of!(Test, Things.stuff),
                size: size_of::<u64>(),
                align: align_of::<u64>(),
            },
        ];
        let stuff = vec![
            Field {
                offset: offset_of!(Test, Stuff.what),
                size: size_of::<u8>(),
                align: align_of::<u8>(),
            },
            Field {
                offset: offset_of!(Test, Stuff.ever),
                size: size_of::<u16>(),
                align: align_of::<u16>(),
            },
        ];
        let this = vec![Field {
            offset: offset_of!(Test, This.0),
            size: size_of::<u64>(),
            align: align_of::<u64>(),
        }];

        dbg!(things);
        dbg!(stuff);
        dbg!(this);
    }

    fn dedup_and_sort() {
        const ORIGINAL: [usize; 10] = [8, 2, 4, 8, 4, 2, 2, 2, 8, 0];
        const DEDUPED: [Option<usize>; 10] = dedup(ORIGINAL);
        const COUNT: usize = count_some(DEDUPED);
        const SORTED: [usize; COUNT] = first_n_sorted(DEDUPED);
        println!("{ORIGINAL:?}");
        println!("{DEDUPED:?}");
        println!("{COUNT:?}");
        println!("{SORTED:?}");
    }
}
