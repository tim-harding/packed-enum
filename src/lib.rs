#![allow(incomplete_features)]

mod byte_vec;

mod pack;

pub use pack::Pack;
pub use packed_enum_derive::Packable;

pub trait Packable {
    const VARIANT_COUNT: usize;

    type Variant: Variant;
    type Ref<'a>;
    type Mut<'a>;

    fn variant(&self) -> Self::Variant;
    fn write(self, dst: *mut u8);
    fn read(variant: Self::Variant, data: *const u8) -> Self;
    fn read_ref<'a>(variant: Self::Variant, data: *const u8) -> Self::Ref<'a>;
    fn read_mut<'a>(variant: Self::Variant, data: *mut u8) -> Self::Mut<'a>;
}

pub trait Variant {
    fn as_index(&self) -> usize;
    fn size_align(&self) -> (usize, usize);
}

/*
/// Creates a [`Pack`] containing the arguments.
///
/// `pack!` allows [`Pack`]s to be defined with the same syntax as array
/// expressions. There are two forms of this macro:
///
/// - Create a [`Pack`] containing a given list of elements:
/// ```
/// # use packed_enum::{EnumInfo, pack};
/// # #[derive(EnumInfo, Debug, PartialEq, Copy, Clone)]
/// # enum Foo { A(bool), B(u8) }
/// let pack = pack![Foo::A(true), Foo::B(1)];
/// # assert_eq!(pack.len(), 2);
/// ```
///
/// - Create a [`Pack`] from a given element and size:
///
/// ```
/// # use packed_enum::{EnumInfo, pack};
/// # #[derive(EnumInfo, Debug, PartialEq, Copy, Clone)]
/// # enum Foo { A(bool), B(u8) }
/// let pack = pack![Foo::A(false); 2];
/// # assert_eq!(pack, pack![Foo::A(false), Foo::A(false)]);
/// ```
#[macro_export]
macro_rules! pack {
    () => {
        $crate::Pack::new()
    };

    ($elem:expr; $n:expr) => {{
        let elem = $elem;
        let mut out = $crate::Pack::new();
        let mut i = 2;
        while i < $n {
            out.push(elem.clone());
        }
        out.push(elem);
        out
    }};

    ($($xs:expr),* $(,)?) => {
        {
            let mut out = $crate::Pack::new();
            $(
            out.push($xs);
            )*
            out
        }
    };
}
*/
