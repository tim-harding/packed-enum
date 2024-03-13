#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod byte_vec;

mod packed;

pub use packed::Packed;
pub use packed_enum_derive::EnumInfo;

pub trait EnumInfo {
    const SIZES: &'static [usize];
    const ALIGNS: &'static [usize];
    const SIZES_COUNT: usize = {
        let sizes = Self::SIZES;
        let mut prev_largest = 0;
        let mut count = 0;
        loop {
            let mut next_largest = usize::MAX;

            let mut i = 0;
            while i < sizes.len() {
                let size = sizes[i];
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

    type Variant;

    fn variant(&self) -> Self::Variant;

    fn read(variant: Self::Variant, data: *const u8) -> Self;

    fn write(self, dst: *mut u8);
}
