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
use syn::{parse_macro_input, Data, DataEnum, DeriveInput, Field, Fields, Variant};

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

    let module = format_ident!("{}_types", to_snake_case(&ident.to_string()));

    let (arm_ignore, (arm_variables, variant_idents)): (Vec<_>, (Vec<_>, Vec<_>)) = e
        .variants
        .iter()
        .map(|variant| {
            let arm_ignore = arm_ignore(variant);
            let arm_variables = arm_variables(variant);
            (arm_ignore, (arm_variables, &variant.ident))
        })
        .collect();

    let construct = read_all(&ident, &module, &e);
    let variant_defs: Orm<Vec<_>> = e.variants.iter().map(variant_defs).collect();
    let (variant_own, variant_ref, variant_mut) = variant_defs.into_tuple();
    let (construct_own, construct_ref, construct_mut) = construct.into_tuple();
    let variant_count = e.variants.len();

    let out = quote! {
        mod #module {
            #(#variant_own)*

            pub enum Ref {
                #(#variant_ref),*
            }

            pub enum Mut {
                #(#variant_mut),*
            }

            #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
            pub enum Variant {
                #(#variant_idents,)*
            }

            impl ::packed_enum::Variant for Variant {
                fn as_index(&self) -> usize {
                    *self as usize
                }

                fn size_align(&self) -> (usize, usize) {
                    match self {
                        #(
                        Self::#variant_idents => (
                            ::std::mem::size_of ::<#module::#variant_idents>(),
                            ::std::mem::align_of::<#module::#variant_idents>(),
                        ),
                        )*
                    }
                }
            }
        }

        impl ::packed_enum::Packable for #ident {
            const VARIANT_COUNT: usize = #variant_count;

            type Variant = #module::Variant;
            type Ref<'a> = #module::Ref<'a>;
            type Mut<'a> = #module::Mut<'a>;

            fn variant(&self) -> Self::Variant {
                match self {
                    #(
                    #ident::#variant_idents #arm_ignore => Self::Variant::#variant_idents,
                    )*
                }
            }

            fn read(variant: Self::Variant, data: *const u8) -> Self {
                match self.variant() {
                    #(
                    #module::Variant::#variant_idents => {
                        let ptr = data.cast::<#module::#variant_idents>();
                        let construct_source = unsafe { ptr.as_ref().unwrap_unchecked() };
                        #construct_own
                    },
                    )*
                }
            }

            fn read_ref<'a>(variant: Self::Variant, data: *const u8) -> Self::Ref<'a> {
                match self.variant() {
                    #(
                    #module::Variant::#variant_idents => {
                        let ptr = data.cast::<#module::#variant_idents>();
                        let construct_source = unsafe { ptr.as_ref().unwrap_unchecked() };
                        #construct_ref
                    },
                    )*
                }
            }

            fn read_mut<'a>(variant: Self::Variant, data: *const u8) -> Self::Mut<'a> {
                match self.variant() {
                    #(
                    #module::Variant::#variant_idents => {
                        let ptr = data.cast::<#module::#variant_idents>();
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

    // Debugging utility. Sometimes `cargo expand` doesn't actually show the macro output if we don't
    // produce a valid token sequence. To show only the macro expansion, use
    // cargo build 2>/dev/null | bat --language rust
    println!("{}", out);

    Ok(out)
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

fn read_all(enom: &Ident, module: &Ident, e: &DataEnum) -> Orm<Vec<TokenStream2>> {
    e.variants
        .iter()
        .map(|variant| read(enom, module, variant))
        .collect()
}

fn read(enom: &Ident, module: &Ident, variant: &Variant) -> Orm<TokenStream2> {
    let Variant { ident, fields, .. } = variant;
    if fields.is_empty() {
        read_empty(enom, ident)
    } else {
        read_full(enom, module, ident, fields)
    }
}

fn read_empty(enom: &Ident, variant: &Ident) -> Orm<TokenStream2> {
    Orm::new(
        quote! { #enom::#variant },
        quote! { #enom::#variant },
        quote! { #enom::#variant },
    )
}

fn read_full(enom: &Ident, module: &Ident, variant: &Ident, fields: &Fields) -> Orm<TokenStream2> {
    let (read_own, read_ref, read_mut) = field_reads(module, variant, fields).into_tuple();
    Orm::new(
        quote! { #enom::#variant { #(#read_own)* } },
        quote! { #enom::#variant { #(#read_ref)* } },
        quote! { #enom::#variant { #(#read_mut)* } },
    )
}

fn field_reads(module: &Ident, variant: &Ident, fields: &Fields) -> Orm<Vec<TokenStream2>> {
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| field_read(module, variant, field, i))
        .collect()
}

fn field_read(module: &Ident, variant: &Ident, field: &Field, i: usize) -> Orm<TokenStream2> {
    let field_ident = IdentOrIndex::from_ident_index(&field.ident, i);
    let offset = quote! { ptr.byte_offset(offset_of!(#module::#variant, #field_ident)) };
    Orm::new(
        quote! { unsafe { #offset.read()             } },
        quote! { unsafe { #offset.as_ref_unchecked() } },
        quote! { unsafe { #offset.as_mut_unchecked() } },
    )
}

fn variant_defs(variant: &Variant) -> Orm<TokenStream2> {
    let Variant { ident, fields, .. } = variant;

    let fields_orm: Orm<Vec<_>> = fields.iter().map(field_orm).collect();
    let (fields_own, fields_ref, fields_mut) = fields_orm.into_tuple();

    if fields.is_empty() {
        Orm::new(
            quote! { pub struct #ident; },
            quote! { #ident },
            quote! { #ident },
        )
    } else if is_tuple(fields) {
        Orm::new(
            quote! { pub struct #ident (#(#fields_own),*); },
            quote! { #ident(#(#fields_ref),*) },
            quote! { #ident(#(#fields_mut),*) },
        )
    } else {
        Orm::new(
            quote! { pub struct #ident { #(#fields_own),* } },
            quote! { #ident { #(#fields_ref),* } },
            quote! { #ident { #(#fields_mut),* } },
        )
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
        .is_none()
}
