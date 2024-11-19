#![allow(unused)]

use packed_enum::EnumInfo;

#[derive(EnumInfo, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Test {
    A(u8, u8, u8, u8),
    B(u16, u16),
    C(u32),
    D(u8, u8, u8, u8, u8, u8, u8, u8),
    E(u16, u16, u16, u16),
    F(u32, u32),
    G(u64),

    I(u8, u16, u32),
    J(u16, u32),

    H,
}

#[cfg(test)]
mod tests {
    use super::*;
    use packed_enum::Pack;

    #[test]
    fn packed() {
        let expected = [Test::A(1, 2, 3, 4), Test::B(5, 6), Test::H, Test::G(7)];
        let mut packed = Pack::new();
        for el in expected {
            packed.push(el);
        }
        for expected in expected.into_iter().rev() {
            assert_eq!(Some(expected), packed.pop());
        }
    }
}
