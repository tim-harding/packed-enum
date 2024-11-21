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

    let variant_count = e.variants.len();
    let module = format_ident!("{}_types", to_snake_case(&ident.to_string()));
    let variant_idents = variant_idents(&e);
    let arm_ignore = arm_ignore_all(&e);
    let arm_variables = arm_variables_all(&e);
    let construct_struct = construct_struct_all(&module, &e);
    let (read_own, read_ref, read_mut) = read_all(&ident, &module, &e).into_tuple();
    let (defs_own, defs_ref, defs_mut) = defs_all(&e).into_tuple();

    let out = quote! {
        mod #module {
            #(#defs_own)*

            pub enum Ref<'a> {
                #(#defs_ref),*
            }

            pub enum Mut<'a> {
                #(#defs_mut),*
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
                            ::std::mem::size_of ::<#variant_idents>(),
                            ::std::mem::align_of::<#variant_idents>(),
                        ),
                        )*
                    }
                }
            }
        }

        #[automatically_derived]
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

            unsafe fn read(variant: Self::Variant, data: *const u8) -> Self {
                match variant {
                    #( #module::Variant::#variant_idents => { #read_own } ),*
                }
            }

            unsafe fn read_ref<'a>(variant: Self::Variant, data: *const u8) -> Self::Ref<'a> {
                match variant {
                    #( #module::Variant::#variant_idents => { #read_ref } ),*
                }
            }

            unsafe fn read_mut<'a>(variant: Self::Variant, data: *mut u8) -> Self::Mut<'a> {
                match variant {
                    #( #module::Variant::#variant_idents => { #read_mut } ),*
                }
            }

            unsafe fn write(self, dst: *mut u8) {
                let me = ::std::mem::ManuallyDrop::new(self);
                let me = <::std::mem::ManuallyDrop<Self> as ::std::ops::Deref>::deref(&me);
                let variant = <Self as ::packed_enum::Packable>::variant(me);
                let (size, align) = ::packed_enum::Variant::size_align(&variant);
                let bytes = size.max(align);
                match me {
                    #(
                    #ident::#variant_idents #arm_variables => {
                        let strukt = ::std::mem::ManuallyDrop::new(#construct_struct);
                        let strukt = <::std::mem::ManuallyDrop<#module::#variant_idents> as ::std::ops::Deref>::deref(&strukt);
                        let src = ::std::ptr::from_ref(strukt).cast();
                        unsafe {
                            ::std::ptr::copy_nonoverlapping(src, dst, bytes);
                        }
                    },
                    )*
                };
            }
        }
    };

    // Debugging utility. Sometimes `cargo expand` doesn't actually show the macro output if we don't
    // produce a valid token sequence. To show only the macro expansion, use
    // cargo build 2>/dev/null | rustfmt | bat --language rust
    // println!("{}", out);

    Ok(out)
}

fn construct_struct_all(module: &Ident, e: &DataEnum) -> Vec<TokenStream2> {
    e.variants
        .iter()
        .map(|variant| construct_struct(module, variant))
        .collect()
}

fn construct_struct(module: &Ident, variant: &Variant) -> TokenStream2 {
    let Variant { ident, fields, .. } = variant;
    let field_idents = field_idents(fields);
    let field_variables = field_variables(fields);
    quote! {
        #module::#ident {
            #(
            #field_idents: {
                let ptr = ::std::ptr::from_ref(#field_variables);
                unsafe { ptr.read() }
            }
            ),*
        }
    }
}

fn field_idents(fields: &Fields) -> Vec<IdentOrIndex> {
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| IdentOrIndex::from_ident_index(&field.ident, i))
        .collect()
}

fn variant_idents(e: &DataEnum) -> Vec<&Ident> {
    e.variants.iter().map(|variant| &variant.ident).collect()
}

fn arm_ignore_all(e: &DataEnum) -> Vec<VariantKind> {
    e.variants.iter().map(arm_ignore).collect()
}

fn arm_ignore(variant: &Variant) -> VariantKind {
    match variant.fields.iter().next() {
        Some(Field { ident: Some(_), .. }) => VariantKind::Struct,
        Some(Field { ident: None, .. }) => VariantKind::Tuple(variant.fields.len()),
        None => VariantKind::Empty,
    }
}

fn arm_variables_all(e: &DataEnum) -> Vec<TokenStream2> {
    e.variants.iter().map(arm_variables).collect()
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
        quote! { #enom    ::#variant },
        quote! { Self::Ref::#variant },
        quote! { Self::Mut::#variant },
    )
}

fn read_full(enom: &Ident, module: &Ident, variant: &Ident, fields: &Fields) -> Orm<TokenStream2> {
    let (read_own, read_ref, read_mut) = field_reads(module, variant, fields).into_tuple();
    Orm::new(
        quote! { #enom    ::#variant { #(#read_own),* } },
        quote! { Self::Ref::#variant { #(#read_ref),* } },
        quote! { Self::Mut::#variant { #(#read_mut),* } },
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
    let Field { ident, ty, .. } = field;
    let field_ident = IdentOrIndex::from_ident_index(ident, i);
    let offset = quote! {
        data.byte_offset(
            ::std::mem::offset_of!(#module::#variant, #field_ident) as isize,
        ).cast::<#ty>()
    };
    // TODO: Use as_ref_unchecked and as_mut_unchecked when stable
    Orm::new(
        quote! { #field_ident: unsafe { #offset.read()                      } },
        quote! { #field_ident: unsafe { #offset.as_ref().unwrap_unchecked() } },
        quote! { #field_ident: unsafe { #offset.as_mut().unwrap_unchecked() } },
    )
}

fn defs_all(e: &DataEnum) -> Orm<Vec<TokenStream2>> {
    e.variants.iter().map(variant_defs).collect()
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
            quote! { pub #ident:         #ty },
            quote! {     #ident: &'a     #ty },
            quote! {     #ident: &'a mut #ty },
        ),
        None => Orm::new(
            quote! { pub         #ty },
            quote! {     &'a     #ty },
            quote! {     &'a mut #ty },
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
