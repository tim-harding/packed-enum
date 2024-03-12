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
    H,
}

#[cfg(test)]
mod tests {
    use packed_enum::Packed;

    use super::*;

    #[test]
    fn sizes_count() {
        assert_eq!(Test::SIZES_COUNT, 2);
        assert_eq!(Packed::<Test>::SIZES, [4, 8]);
        assert_eq!(
            Packed::<Test>::BUCKET,
            [
                Some(0),
                Some(0),
                Some(0),
                Some(1),
                Some(1),
                Some(1),
                Some(1),
                None,
            ]
        );
    }
}
