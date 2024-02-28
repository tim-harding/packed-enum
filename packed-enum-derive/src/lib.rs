use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{parse_macro_input, Data, DeriveInput, Variant};

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
        vis: _,
        ident,
        generics: _,
        attrs: _,
    } = input;
    let strukts_mod = format_ident!("{ident}_variants");

    match data {
        Data::Enum(e) => {
            let strukts = e.variants.into_iter().map(|variant| {
                let Variant {
                    ident: variant_ident,
                    fields,
                    discriminant: _,
                    attrs: _,
                } = variant;

                match fields {
                    syn::Fields::Named(fields) => quote! { #variant_ident #fields },
                    syn::Fields::Unnamed(fields) => quote! { #variant_ident #fields; },
                    syn::Fields::Unit => quote! { #variant_ident; },
                }
            });

            Ok(quote! {
                #[automatically_derived]
                mod #strukts_mod {
                    #(pub struct #strukts)*
                }
            })
        }

        Data::Struct(_) | Data::Union(_) => Err(PackedError::NotAnEnum),
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
