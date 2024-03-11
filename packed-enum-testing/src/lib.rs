#![feature(offset_of_nested)]
#![feature(offset_of_enum)]
#![cfg(test)]
#![allow(unused)]

use packed_enum::EnumInfo;

#[derive(EnumInfo)]
enum Test {
    Things { hello: u16, world: u8, stuff: u64 },
    Stuff { what: u8, ever: u16 },
    This(u64),
}
