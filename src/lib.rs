#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod byte_vec;

mod packed;
use core::panic;

pub use packed::Packed;

pub use packed_enum_derive::EnumInfo;

pub trait EnumInfo {
    const VARIANTS: &'static [&'static [VariantField]];
    const VARIANT_COUNT: usize = Self::VARIANTS.len();
    const SIZES_COUNT: usize = {
        let variants = Self::VARIANTS;
        let mut prev_largest = 0;
        let mut count = 0;
        loop {
            let mut next_largest = usize::MAX;

            let mut i = 0;
            while i < variants.len() {
                let variant = variants[i];
                let size = variant_size(variant);
                if size > prev_largest && size < next_largest {
                    next_largest = size;
                }
                i += 1;
            }

            if next_largest == usize::MAX {
                break;
            }
            prev_largest = next_largest;
            count += 1;
        }
        count
    };

    fn variant_index(&self) -> usize;

    fn make_variant(variant_index: usize, data: *const u8) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VariantField {
    pub size: usize,
    pub align: usize,
    pub offset: usize,
}

const fn variant_size(fields: &[VariantField]) -> usize {
    let mut min = usize::MAX;
    let mut max = 0;
    let mut max_align = 0;

    let mut i = 0;
    while i < fields.len() {
        let field = &fields[i];

        let lo = field.offset;
        min = if min < lo { min } else { lo };

        let hi = field.offset + field.size;
        max = if max > hi { max } else { hi };

        max_align = if max_align > field.align {
            max_align
        } else {
            field.align
        };

        i += 1;
    }

    let size = match max.checked_sub(min) {
        Some(size) => size,
        None => 0,
    };

    if max_align > size {
        panic!("Expected size to exceed alignment");
    }

    // Taken from Layout::padding_needed_for
    let size_rounded_up = size.wrapping_add(max_align).wrapping_sub(1) & !max_align.wrapping_sub(1);
    let padding = size_rounded_up.wrapping_sub(size);

    size + padding
}
