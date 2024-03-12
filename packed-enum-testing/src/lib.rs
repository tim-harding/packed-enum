#![feature(offset_of_nested)]
#![feature(offset_of_enum)]
#![allow(unused)]

use packed_enum::EnumInfo;

#[derive(EnumInfo)]
enum Test {
    A(u8, u8, u8, u8),
    B(u16, u16),
    C(u32),
    D(u8, u8, u8, u8, u8, u8, u8, u8),
    E(u16, u16, u16, u16),
    F(u32, u32),
    G(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_count() {
        dbg!(Test::VARIANTS);
        assert_eq!(Test::SIZES_COUNT, 2);
    }
}
