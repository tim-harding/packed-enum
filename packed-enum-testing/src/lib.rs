#![allow(unused)]

use packed_enum::Packable;

#[derive(Packable, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Test {
    A(u8, u8, u8, u8),
    B { foo: u16, bar: u16 },
    C,
}

#[cfg(test)]
mod tests {
    use super::*;
    use packed_enum::Pack;

    #[test]
    fn packed() {
        let expected = [Test::A(1, 2, 3, 4), Test::B { foo: 5, bar: 6 }, Test::C];
        let mut packed = Pack::new();
        for el in expected {
            packed.push(el);
        }
        for expected in expected.into_iter().rev() {
            assert_eq!(Some(expected), packed.pop());
        }
    }
}
