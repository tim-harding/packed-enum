mod orm;
use orm::Orm;

mod packed_error;
use packed_error::PackedError;

mod variant_kind;
use variant_kind::VariantKind;

mod ident_or_index;
use ident_or_index::IdentOrIndex;

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Variant};

#[proc_macro_derive(Packable)]
pub fn packable(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let span = input.ident.span();
    match packable_inner(input) {
        Ok(tokens) => tokens,
        Err(PackedError::Syn(e)) => e.into_compile_error(),
        Err(PackedError::NotAnEnum) => quote_spanned! {
            span => compile_error!("Packable only applies to enums");
        },
    }
    .into()
}

fn packable_inner(input: DeriveInput) -> Result<TokenStream2, PackedError> {
    let DeriveInput { data, ident, .. } = input;
    let Data::Enum(e) = data else {
        return Err(PackedError::NotAnEnum);
    };

    #[allow(clippy::type_complexity)]
    let (construct, (strukts, (arm_ignore, (arm_variables, variant_idents)))): (
        Orm<Vec<_>>,
        (Vec<_>, (Vec<_>, (Vec<_>, Vec<_>))),
    ) = e
        .variants
        .iter()
        .map(|variant| {
            let construct = constructors(&ident, variant);
            let strukts = struct_definitions(variant);
            let arm_ignore = arm_ignore(variant);
            let arm_variables = arm_variables(variant);
            (
                construct,
                (strukts, (arm_ignore, (arm_variables, &variant.ident))),
            )
        })
        .collect();

    let (construct_own, construct_ref, construct_mut) = construct.into_tuple();
    let strukt_module = format_ident!("{}_structs", to_snake_case(&ident.to_string()));

    Ok(quote! {
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
    })
}

fn arm_ignore(variant: &Variant) -> VariantKind {
    match variant.fields.iter().next() {
        Some(Field { ident: Some(_), .. }) => VariantKind::Struct,
        Some(Field { ident: None, .. }) => VariantKind::Tuple(variant.fields.len()),
        None => VariantKind::Empty,
    }
}

fn arm_variables(variant: &Variant) -> TokenStream2 {
    let Variant { fields, .. } = variant;
    let field_variables = field_variables(&variant.fields);
    if fields.is_empty() {
        quote! {}
    } else if is_tuple(fields) {
        quote! { (#(#field_variables),*) }
    } else {
        let field_idents = fields.iter().map(|field| &field.ident);
        quote! { { #( #field_idents: #field_variables,)* } }
    }
}

fn field_variables(fields: &Fields) -> Vec<Ident> {
    fields
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("field_{}", i))
        .collect()
}

fn constructors(enom: &Ident, variant: &Variant) -> Orm<TokenStream2> {
    let Variant { ident, fields, .. } = variant;
    let ident = Orm::from_ident(ident);
    if fields.is_empty() {
        constructor_empty(enom, &ident)
    } else {
        constructor_full(enom, &ident, fields)
    }
}

fn constructor_empty(enom: &Ident, ident: &Orm<Ident>) -> Orm<TokenStream2> {
    let (ident_own, ident_ref, ident_mut) = ident.as_ref().into_tuple();
    Orm::new(
        quote! { #enom::#ident_own },
        quote! { #enom::#ident_ref },
        quote! { #enom::#ident_mut },
    )
}

fn constructor_full(enom: &Ident, ident: &Orm<Ident>, fields: &Fields) -> Orm<TokenStream2> {
    let (setters_own, setters_ref, setters_mut) = setters(fields).into_tuple();
    let (ident_own, ident_ref, ident_mut) = ident.as_ref().into_tuple();
    Orm::new(
        quote! { #enom::#ident_own { #(#setters_own)* } },
        quote! { #enom::#ident_ref { #(#setters_ref)* } },
        quote! { #enom::#ident_mut { #(#setters_mut)* } },
    )
}

fn setters(fields: &Fields) -> Orm<Vec<TokenStream2>> {
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| setter(field, i))
        .collect()
}

fn setter(field: &Field, i: usize) -> Orm<TokenStream2> {
    let field_ident = IdentOrIndex::from_ident_index(&field.ident, i);
    Orm::new(
        quote! {
            #field_ident: {
                let ptr = ::std::ptr::from_ref(&construct_source.#field_ident);
                unsafe { ptr.read() }
            },
        },
        quote! { #field_ident: &    construct_source.#field_ident, },
        quote! { #field_ident: &mut construct_source.#field_ident, },
    )
}

fn struct_definitions(variant: &Variant) -> TokenStream2 {
    let Variant { ident, fields, .. } = variant;

    let ident = Orm::from_ident(ident);
    let (ident_own, ident_ref, ident_mut) = ident.into_tuple();

    let fields_orm: Orm<Vec<_>> = fields.iter().map(field_orm).collect();
    let (fields_own, fields_ref, fields_mut) = fields_orm.into_tuple();

    if fields.is_empty() {
        quote! {
            pub struct #ident_own;
            pub struct #ident_ref;
            pub struct #ident_mut;
        }
    } else if is_tuple(fields) {
        quote! {
            pub struct #ident_own    (#(#fields_own),*);
            pub struct #ident_ref<'a>(#(#fields_ref),*);
            pub struct #ident_mut<'a>(#(#fields_mut),*);
        }
    } else {
        quote! {
            pub struct #ident_own     { #(#fields_own),* }
            pub struct #ident_ref<'a> { #(#fields_ref),* }
            pub struct #ident_mut<'a> { #(#fields_mut),* }
        }
    }
}

fn field_orm(field: &Field) -> Orm<TokenStream2> {
    let Field { ident, ty, .. } = field;
    match ident {
        Some(ident) => Orm::new(
            quote! { pub #ident:      #ty },
            quote! { pub #ident: &    #ty },
            quote! { pub #ident: &mut #ty },
        ),
        None => Orm::new(
            quote! { pub      #ty },
            quote! { pub &    #ty },
            quote! { pub &mut #ty },
        ),
    }
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

fn is_tuple(fields: &Fields) -> bool {
    fields
        .iter()
        .next()
        .and_then(|field| field.ident.as_ref())
        .is_some()
}
