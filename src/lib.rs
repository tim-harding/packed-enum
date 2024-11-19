#![allow(incomplete_features)]

mod byte_vec;

mod packed;

pub use packed::Packed;
pub use packed_enum_derive::EnumInfo;

pub trait EnumInfo {
    const SIZES: &'static [usize];
    const ALIGNS: &'static [usize];
    type Variant: AsIndex;
    fn variant(&self) -> Self::Variant;
    fn read(variant: Self::Variant, data: *const u8) -> Self;
    fn write(self, dst: *mut u8);
}

pub trait AsIndex {
    fn as_index(&self) -> usize;
}
