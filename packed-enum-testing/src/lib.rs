#![cfg(test)]

use packed_enum::Packed;

#[allow(unused)]
#[derive(Packed)]
enum Test {
    Things,
    Stuff(u8),
    Foo { bar: u16, baz: u32 },
}

#[test]
fn stuff() {}
