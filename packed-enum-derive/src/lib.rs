use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Variant};

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
    let discriminants = format_ident!("{ident}Discriminants");

    match data {
        Data::Enum(e) => {
            let (fields, ident_variant): (Vec<_>, Vec<_>) = e
                .variants
                .into_iter()
                .map(|variant| {
                    let Variant {
                        ident,
                        fields,
                        discriminant: _,
                        attrs: _,
                    } = variant;
                    (fields, ident)
                })
                .unzip();

            let ident_variant_snake: Vec<_> = ident_variant
                .iter()
                .map(|ident| to_snake_ident(&ident))
                .collect();

            let strukt_definitions: Vec<_> = fields
                .iter()
                .zip(&ident_variant)
                .map(|(fields, ident_variant)| match fields {
                    Fields::Named(fields) => {
                        let definition = fields.named.iter().map(|field| {
                            let Field { ident, ty, .. } = field;
                            quote! {
                                pub #ident: #ty,
                            }
                        });
                        quote! {
                            #ident_variant {
                                #(#definition)*
                            }
                        }
                    }

                    Fields::Unnamed(fields) => {
                        let definition = fields.unnamed.iter().map(|field| {
                            let Field { ty, .. } = field;
                            quote! {
                                pub #ty
                            }
                        });
                        quote! {
                            #ident_variant ( #(#definition),* );
                        }
                    }

                    Fields::Unit => quote! { #ident_variant; },
                })
                .collect();

            let shapes: Vec<_> = fields
                .iter()
                .map(|fields| match fields {
                    Fields::Named(fields) => {
                        let field_idents = fields.named.iter().map(|field| &field.ident);
                        quote! {
                            {
                                #(#field_idents,)*
                            }
                        }
                    }

                    Fields::Unnamed(fields) => {
                        let field_idents = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(index, _)| format_ident!("f_{}", index));
                        quote! {
                            ( #(#field_idents),* )
                        }
                    }

                    Fields::Unit => quote! {},
                })
                .collect();

            Ok(quote! {
                #[automatically_derived]
                mod #strukts_mod {
                    #(pub struct #strukt_definitions)*
                }

                #vis enum #discriminants {
                    #( #ident_variant, )*
                }

                #[automatically_derived]
                #[derive(Default)]
                #vis struct #enum_vec {
                    // Field index, index within field
                    indices: Vec<(#discriminants, usize)>,
                    #(
                        #ident_variant_snake: Vec<#strukts_mod::#ident_variant>,
                    )*
                }

                #[automatically_derived]
                impl #enum_vec {
                    pub fn new() -> Self {
                        Self::default()
                    }

                    pub fn push(&mut self, element: #ident) {
                        match element {
                            #(
                                #ident::#ident_variant #shapes => {
                                    let strukt = #strukts_mod::#ident_variant #shapes;
                                    let index = self.#ident_variant_snake.len();
                                    self.#ident_variant_snake.push(strukt);
                                    self.indices.push((#discriminants::#ident_variant, index));
                                }
                            )*
                        }
                    }

                    pub fn pop(&mut self) -> Option<#ident> {
                        let (field_index, _) = self.indices.pop()?;
                        match field_index {
                            #(
                                #discriminants::#ident_variant => {
                                    let #strukts_mod::#ident_variant #shapes =
                                        self.#ident_variant_snake.pop()?;
                                    Some(#ident::#ident_variant #shapes)
                                }
                            )*
                        }
                    }
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
