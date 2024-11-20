use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, Data, DataEnum, DeriveInput, Field, Index, Variant};

#[proc_macro_derive(Packable)]
pub fn packed(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let span = input.ident.span();
    match packed_inner(input) {
        Ok(tokens) => tokens,
        Err(e) => match e {
            PackedError::NotAnEnum => quote_spanned! {
                span => compile_error!("Packable only applies to enums");
            },
            PackedError::Syn(e) => e.into_compile_error(),
        },
    }
    .into()
}

fn packed_inner(input: DeriveInput) -> Result<TokenStream2, PackedError> {
    let DeriveInput {
        data,
        ident,
        vis: _,
        generics: _,
        attrs: _,
    } = input;

    let Data::Enum(e) = data else {
        return Err(PackedError::NotAnEnum);
    };

    let snake = to_snake_case(&ident.to_string());
    let strukt_module = format_ident!("{}_strukts", snake);

    let strukts = struct_definitions(&e);

    let (construct_own, (construct_ref, construct_mut)): (Vec<_>, (Vec<_>, Vec<_>)) = e
        .variants
        .iter()
        .map(|variant| {
            let Variant {
                ident: variant_ident_own,
                fields,
                ..
            } = variant;
            let variant_ident_ref = ident_ref(variant_ident_own);
            let variant_ident_mut = ident_mut(variant_ident_own);

            if fields.is_empty() {
                let a = quote! { #ident::#variant_ident_own };
                let b = quote! { #ident::#variant_ident_ref };
                let c = quote! { #ident::#variant_ident_mut };
                return (a, (b, c));
            }

            let (setters_own, (setters_ref, setters_mut)): (Vec<_>, (Vec<_>, Vec<_>)) = fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let Field {
                        ident: field_ident, ..
                    } = field;
                    let field_ident = IdentOrIndex::from_ident_index(field_ident, i);
                    let field_own = quote! {
                        #field_ident: {
                            let ptr = ::std::ptr::from_ref(&construct_source.#field_ident);
                            unsafe { ptr.read() }
                        },
                    };
                    let field_ref = quote! {
                        #field_ident: &construct_source.#field_ident,
                    };
                    let field_mut = quote! {
                        #field_ident: &mut construct_source.#field_ident,
                    };
                    (field_own, (field_ref, field_mut))
                })
                .unzip();

            let construct_own = quote! { #ident::#variant_ident_own { #(#setters_own)* } };
            let construct_ref = quote! { #ident::#variant_ident_own { #(#setters_ref)* } };
            let construct_mut = quote! { #ident::#variant_ident_own { #(#setters_mut)* } };
            (construct_own, (construct_ref, construct_mut))
        })
        .unzip();

    let field_variable_idents: Vec<Vec<_>> = e
        .variants
        .iter()
        .map(|variant| {
            variant
                .fields
                .iter()
                .enumerate()
                .map(|(i, _)| format_ident!("field_{}", i))
                .collect()
        })
        .collect();

    let (constructor_own, (constructor_ref, constructor_mut)): (Vec<_>, (Vec<_>, Vec<_>)) = e
        .variants
        .iter()
        .zip(&field_variable_idents)
        .map(|(variant, field_variables)| {
            let Variant {
                ident: variant_ident_own,
                fields,
                ..
            } = variant;
            let variant_ident_ref = ident_ref(variant_ident_own);
            let variant_ident_mut = ident_mut(variant_ident_own);

            if fields.is_empty() {
                let a = quote! { #strukt_module::#variant_ident_own };
                let b = quote! { #strukt_module::#variant_ident_ref };
                let c = quote! { #strukt_module::#variant_ident_mut };
                return (a, (b, c));
            }

            let (field_setters_own, (field_setters_ref, field_setters_mut)): (
                Vec<_>,
                (Vec<_>, Vec<_>),
            ) = fields
                .iter()
                .zip(field_variables)
                .enumerate()
                .map(|(i, (field, field_variable))| {
                    let Field {
                        ident: field_ident, ..
                    } = field;
                    let field_ident = IdentOrIndex::from_ident_index(field_ident, i);
                    let construct_own = quote! {
                        #field_ident: {
                            let ptr = ::std::ptr::from_ref(#field_variable);
                            unsafe { ptr.read() }
                        },
                    };
                    let construct_ref = quote! {
                        #field_ident: & #field_variable,
                    };
                    let construct_mut = quote! {
                        #field_ident: &mut #field_variable,
                    };
                    (construct_own, (construct_ref, construct_mut))
                })
                .unzip();

            let a = quote! { #strukt_module::#variant_ident_own { #(#field_setters_own)* } };
            let b = quote! { #strukt_module::#variant_ident_ref { #(#field_setters_ref)* } };
            let c = quote! { #strukt_module::#variant_ident_mut { #(#field_setters_mut)* } };
            (a, (b, c))
        })
        .unzip();

    let arm_ignore: Vec<_> = e
        .variants
        .iter()
        .map(|variant| match variant.fields.iter().next() {
            Some(field) => match field.ident {
                Some(_) => VariantKind::Struct,
                None => VariantKind::Tuple(variant.fields.len()),
            },
            None => VariantKind::Empty,
        })
        .collect();

    let arm_variables: Vec<_> = e
        .variants
        .iter()
        .zip(field_variable_idents)
        .map(|(variant, field_variables)| {
            if variant.fields.is_empty() {
                return quote! {};
            }

            let is_tuple = variant.fields.iter().next().unwrap().ident.is_none();
            if is_tuple {
                quote! {
                    (#(#field_variables),*)
                }
            } else {
                let field_idents = variant.fields.iter().map(|field| &field.ident);
                quote! {
                    {
                        #(
                        #field_idents: #field_variables,
                        )*
                    }
                }
            }
        })
        .collect();

    let variant_idents: Vec<_> = e.variants.iter().map(|variant| &variant.ident).collect();

    let out = quote! {
        mod #strukt_module {
            #(#strukts)*

            #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
            pub enum Variant {
                #(#variant_idents,)*
            }

            impl ::packed_enum::AsIndex for Variant {
                fn as_index(&self) -> usize {
                    *self as usize
                }
            }
        }

        impl ::packed_enum::Packable for #ident {
            const SIZES: &'static [usize] = &[
                #(::std::mem::size_of::<#strukt_module::#variant_idents>(),)*
            ];

            const ALIGNS: &'static [usize] = &[
                #(::std::mem::align_of::<#strukt_module::#variant_idents>(),)*
            ];

            type Variant = #strukt_module::Variant;
            type Ref = #strukt_module::Ref;
            type Mut = #strukt_module::Mut;

            fn variant(&self) -> Self::Variant {
                match self {
                    #(
                    #ident::#variant_idents #arm_ignore => Self::Variant::#variant_idents,
                    )*
                }
            }

            fn read(data: *const u8) -> Self {
                match self.variant() {
                    #(
                    #strukt_module::Variant::#variant_idents => {
                        let ptr = data.cast::<#strukt_module::#variant_idents>();
                        let construct_source = unsafe { ptr.as_ref().unwrap_unchecked() };
                        #construct_own
                    },
                    )*
                }
            }

            fn read_ref(data: *const u8) -> Self::Ref {
                match self.variant() {
                    #(
                    #strukt_module::Variant::#variant_idents => {
                        let ptr = data.cast::<#strukt_module::#variant_idents>();
                        let construct_source = unsafe { ptr.as_ref().unwrap_unchecked() };
                        #construct_ref
                    },
                    )*
                }
            }

            fn read_mut(data: *const u8) -> Self::Mut {
                match self.variant() {
                    #(
                    #strukt_module::Variant::#variant_idents => {
                        let ptr = data.cast::<#strukt_module::#variant_idents>();
                        let construct_source = unsafe { ptr.as_ref().unwrap_unchecked() };
                        #construct_mut
                    },
                    )*
                }
            }

            fn write(self, dst: *mut u8) {
                let s = ::std::mem::ManuallyDrop::new(self);
                let construct_source = <::std::mem::ManuallyDrop<Self> as ::std::ops::Deref>::deref(&s);
                let strukt = match construct_source {
                    #(
                    #ident::#variant_idents #arm_variables => {
                        let strukt = #construct_own;
                        let src = ::std::ptr::from_ref(&strukt).cast();
                        let count = ::std::mem::size_of_val(&strukt);
                        unsafe {
                            ::std::ptr::copy_nonoverlapping(src, dst, count);
                        }
                    },
                    )*
                };
            }
        }
    };

    Ok(out)
}

fn to_snake_case(s: &str) -> String {
    let mut chars = s.chars();
    let mut out = String::new();
    out.extend(chars.next().iter().flat_map(|c| c.to_lowercase()));
    for c in chars {
        if c.is_lowercase() {
            out.push(c);
        } else {
            out.extend(std::iter::once('_').chain(c.to_lowercase()))
        }
    }
    out
}

enum VariantKind {
    Empty,
    Tuple(usize),
    Struct,
}

impl ToTokens for VariantKind {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            VariantKind::Empty => {}
            VariantKind::Tuple(field_count) => {
                let iter = std::iter::repeat(quote! { _ }).take(*field_count);
                quote! { (#(#iter),*) }.to_tokens(tokens)
            }
            VariantKind::Struct => quote! { { .. } }.to_tokens(tokens),
        }
    }
}

enum IdentOrIndex<'a> {
    Ident(&'a Ident),
    Index(Index),
}

impl<'a> IdentOrIndex<'a> {
    pub fn from_ident_index(ident: &'a Option<Ident>, index: usize) -> Self {
        match ident {
            Some(ident) => Self::Ident(ident),
            None => Self::Index(Index::from(index)),
        }
    }
}

impl<'a> ToTokens for IdentOrIndex<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            IdentOrIndex::Ident(ident) => ident.to_tokens(tokens),
            IdentOrIndex::Index(i) => i.to_tokens(tokens),
        }
    }
}

#[derive(Debug, Clone)]
enum PackedError {
    NotAnEnum,
    Syn(syn::Error),
}

impl From<syn::Error> for PackedError {
    fn from(value: syn::Error) -> Self {
        Self::Syn(value)
    }
}

fn ident_ref(ident: &Ident) -> Ident {
    format_ident!("{}Ref", ident)
}

fn ident_mut(ident: &Ident) -> Ident {
    format_ident!("{}RefMut", ident)
}

struct OwnRefMut {
    o: TokenStream2,
    r: TokenStream2,
    m: TokenStream2,
}

impl OwnRefMut {
    pub const fn new(o: TokenStream2, r: TokenStream2, m: TokenStream2) -> Self {
        Self { o, r, m }
    }

    pub fn into_tuple(self) -> (TokenStream2, TokenStream2, TokenStream2) {
        let Self { o, r, m } = self;
        (o, r, m)
    }

    pub fn into_tuple_nest(self) -> (TokenStream2, (TokenStream2, TokenStream2)) {
        let Self { o, r, m } = self;
        (o, (r, m))
    }
}

impl From<(TokenStream2, TokenStream2, TokenStream2)> for OwnRefMut {
    fn from((a, b, c): (TokenStream2, TokenStream2, TokenStream2)) -> Self {
        Self::new(a, b, c)
    }
}

impl From<OwnRefMut> for (TokenStream2, TokenStream2, TokenStream2) {
    fn from(OwnRefMut { o, r, m }: OwnRefMut) -> Self {
        (o, r, m)
    }
}

struct OwnRefMutVec {
    o: Vec<TokenStream2>,
    r: Vec<TokenStream2>,
    m: Vec<TokenStream2>,
}

impl OwnRefMutVec {
    pub const fn new(o: Vec<TokenStream2>, r: Vec<TokenStream2>, m: Vec<TokenStream2>) -> Self {
        Self { o, r, m }
    }

    pub fn into_tuple(self) -> (Vec<TokenStream2>, Vec<TokenStream2>, Vec<TokenStream2>) {
        let Self { o, r, m } = self;
        (o, r, m)
    }
}

impl FromIterator<OwnRefMut> for OwnRefMutVec {
    fn from_iter<T: IntoIterator<Item = OwnRefMut>>(iter: T) -> Self {
        let (o, (r, m)) = iter.into_iter().map(OwnRefMut::into_tuple_nest).unzip();
        Self { o, r, m }
    }
}

fn struct_definitions(e: &DataEnum) -> Vec<TokenStream2> {
    e.variants
        .iter()
        .map(|variant| {
            let Variant {
                ident: variant_ident_own,
                fields,
                ..
            } = variant;
            let variant_ident_ref = ident_ref(variant_ident_own);
            let variant_ident_mut = ident_mut(variant_ident_own);

            if fields.is_empty() {
                return quote! {
                    pub struct #variant_ident_own;
                };
            }

            let fields_orm: OwnRefMutVec = fields.iter().map(variant_field_orm).collect();
            let (fields_own, fields_ref, fields_mut) = fields_orm.into_tuple();

            let is_tuple = fields.iter().next().unwrap().ident.is_none();
            if is_tuple {
                quote! {
                    pub struct #variant_ident_own    (#(#fields_own),*);
                    pub struct #variant_ident_ref<'a>(#(#fields_ref),*);
                    pub struct #variant_ident_mut<'a>(#(#fields_mut),*);
                }
            } else {
                quote! {
                    pub struct #variant_ident_own     { #(#fields_own),* }
                    pub struct #variant_ident_ref<'a> { #(#fields_ref),* }
                    pub struct #variant_ident_mut<'a> { #(#fields_mut),* }
                }
            }
        })
        .collect()
}

fn variant_field_orm(field: &Field) -> OwnRefMut {
    let Field { ident, ty, .. } = field;
    match ident {
        Some(ident) => (
            quote! { pub #ident:      #ty },
            quote! { pub #ident: &    #ty },
            quote! { pub #ident: &mut #ty },
        ),
        None => (
            quote! { pub      #ty },
            quote! { pub &    #ty },
            quote! { pub &mut #ty },
        ),
    }
    .into()
}
