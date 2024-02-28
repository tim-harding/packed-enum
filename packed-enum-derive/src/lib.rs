use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Variant};

#[proc_macro_derive(Packed)]
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
        vis,
        ident,
        generics: _,
        attrs: _,
    } = input;
    let ident_snake = to_snake_ident(&ident);
    let strukts_mod = format_ident!("{ident_snake}_variants");
    let enum_vec = format_ident!("{ident}Packed");

    match data {
        Data::Enum(e) => {
            let (fields, (ident_variant_snake, ident_variant)): (Vec<_>, (Vec<_>, Vec<_>)) = e
                .variants
                .into_iter()
                .map(|variant| {
                    let Variant {
                        ident,
                        fields,
                        discriminant: _,
                        attrs: _,
                    } = variant;
                    (fields, (to_snake_ident(&ident), ident))
                })
                .unzip();

            let strukts: Vec<_> = fields
                .iter()
                .zip(&ident_variant)
                .map(|(fields, ident_variant)| match fields {
                    Fields::Named(fields) => quote! { #ident_variant #fields },
                    Fields::Unnamed(fields) => quote! { #ident_variant #fields; },
                    Fields::Unit => quote! { #ident_variant; },
                })
                .collect();

            Ok(quote! {
                #[automatically_derived]
                mod #strukts_mod {
                    #(pub struct #strukts)*
                }

                #[automatically_derived]
                #vis struct #enum_vec {
                    #(pub #ident_variant_snake: Vec<#strukts_mod::#ident_variant>,)*
                }
            })
        }

        Data::Struct(_) | Data::Union(_) => Err(PackedError::NotAnEnum),
    }
}

fn to_snake_ident(ident: &Ident) -> Ident {
    Ident::new(&to_snake_case(&ident.to_string()), ident.span())
}

fn to_snake_case(s: &str) -> String {
    let mut chars = s.chars();
    let mut out = String::new();
    if let Some(c) = chars.next() {
        out.extend(c.to_lowercase());
    }
    while let Some(c) = chars.next() {
        if c.is_uppercase() {
            out.push('_');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
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
