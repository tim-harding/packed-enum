#![cfg(test)]
#![allow(unused)]

use packed_enum::Packed;

#[derive(Packed, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Test {
    Things,
    Stuff(u8),
    Foo { bar: u16, baz: u32 },
}

#[test]
fn stuff() {
    let mut v = TestPacked::new();
    v.push(Test::Things);
    v.push(Test::Stuff(10));
    v.push(Test::Foo { bar: 20, baz: 30 });
    assert_eq!(v.pop(), Some(Test::Foo { bar: 20, baz: 30 }));
    assert_eq!(v.pop(), Some(Test::Stuff(10)));
    assert_eq!(v.pop(), Some(Test::Things));
    assert_eq!(v.pop(), None);
}
