use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Field, Index, Variant};

#[proc_macro_derive(EnumInfo)]
pub fn packed(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let span = input.ident.span();
    match packed_inner(input) {
        Ok(tokens) => tokens,
        Err(e) => match e {
            PackedError::NotAnEnum => quote_spanned! {
                span => compile_error!("Packed only applies to enums");
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

    match data {
        Data::Enum(e) => {
            let variant_idents = e.variants.iter().map(|variant| &variant.ident);
            let variant_kinds =
                e.variants
                    .iter()
                    .map(|variant| match variant.fields.iter().next() {
                        Some(field) => match field.ident {
                            Some(_) => VariantKind::Struct,
                            None => VariantKind::Tuple(variant.fields.len()),
                        },
                        None => VariantKind::Empty,
                    });
            let variant_indices = e.variants.iter().enumerate().map(|(i, _)| i);

            let variants = e.variants.iter().map(|variant| {
                let Variant {
                    ident: variant_ident,
                    fields: variant_fields,
                    attrs: _,
                    discriminant: _,
                } = variant;

                let fields_info = variant_fields.iter().enumerate().map(|(i, field)| {
                    let Field {
                        ident: field_ident,
                        ty,
                        attrs: _,
                        vis: _,
                        mutability: _,
                        colon_token: _,
                    } = field;

                    let field_ident = match field_ident {
                        Some(field_ident) => IdentOrIndex::Ident(field_ident),
                        None => IdentOrIndex::Index(Index::from(i)),
                    };

                    quote! {
                        ::packed_enum::VariantField {
                            offset: ::std::mem::offset_of!(#ident, #variant_ident.#field_ident),
                            size: ::std::mem::size_of::<#ty>(),
                            align: ::std::mem::align_of::<#ty>(),
                        }
                    }
                });

                quote! {
                    &[#(#fields_info),*]
                }
            });

            let out = quote! {
                impl ::packed_enum::EnumInfo for #ident {
                    const VARIANTS: &'static [&'static [::packed_enum::VariantField]] = &[
                        #(#variants),*
                    ];

                    fn variant_index(&self) -> usize {
                        match self {
                            #(
                            #ident::#variant_idents #variant_kinds => #variant_indices,
                            )*
                        }
                    }
                }
            };

            Ok(out)
        }

        Data::Struct(_) | Data::Union(_) => Err(PackedError::NotAnEnum),
    }
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
