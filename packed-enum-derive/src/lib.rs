use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Field, Index, Variant};

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

    match data {
        Data::Enum(e) => {
            let snake = to_snake_case(&ident.to_string());
            let strukt_module = format_ident!("{}_strukts", snake);

            let strukts = e.variants.iter().map(|variant| {
                let Variant {
                    ident: variant_ident_own,
                    fields: variant_fields,
                    ..
                } = variant;
                let variant_ident_ref = ident_ref(variant_ident_own);
                let variant_ident_mut = ident_mut(variant_ident_own);

                if variant_fields.is_empty() {
                    return quote! {
                        pub struct #variant_ident_own;
                    };
                }

                let is_tuple = variant_fields.iter().next().unwrap().ident.is_none();

                let (strukt_fields_own, (strukt_fields_ref, strukt_fields_mut)): (
                    Vec<_>,
                    (Vec<_>, Vec<_>),
                ) = variant_fields
                    .iter()
                    .map(|field| {
                        let Field {
                            ident: field_ident,
                            ty: field_ty,
                            ..
                        } = field;
                        let (a, b, c) = match field_ident {
                            Some(field_ident) => (
                                quote! { pub #field_ident:      #field_ty },
                                quote! { pub #field_ident: &    #field_ty },
                                quote! { pub #field_ident: &mut #field_ty },
                            ),
                            None => (
                                quote! { pub      #field_ty },
                                quote! { pub &    #field_ty },
                                quote! { pub &mut #field_ty },
                            ),
                        };
                        (a, (b, c))
                    })
                    .unzip();

                if is_tuple {
                    quote! {
                        pub struct #variant_ident_own(#(#strukt_fields_own),*);
                        pub struct #variant_ident_ref(#(#strukt_fields_ref),*);
                        pub struct #variant_ident_mut(#(#strukt_fields_mut),*);
                    }
                } else {
                    quote! {
                        pub struct #variant_ident_own { #(#strukt_fields_own),* }
                        pub struct #variant_ident_ref { #(#strukt_fields_ref),* }
                        pub struct #variant_ident_mut { #(#strukt_fields_mut),* }
                    }
                }
            });

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

                    let (setters_own, (setters_ref, setters_mut)): (Vec<_>, (Vec<_>, Vec<_>)) =
                        fields.iter().enumerate().map(|(i, field)| {
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
                        }).unzip();

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

            let strukt_constructors: Vec<_> = e
                .variants
                .iter()
                .zip(&field_variable_idents)
                .map(|(variant, field_variables)| {
                    let Variant {
                        ident: variant_ident,
                        fields,
                        ..
                    } = variant;

                    if fields.is_empty() {
                        return quote! {
                            #strukt_module::#variant_ident
                        };
                    }

                    let field_setters: Vec<_> = fields
                        .iter()
                        .zip(field_variables)
                        .enumerate()
                        .map(|(i, (field, field_variable))| {
                            let Field {
                                ident: field_ident, ..
                            } = field;
                            let field_ident = IdentOrIndex::from_ident_index(field_ident, i);
                            quote! {
                                #field_ident: {
                                    let ptr = ::std::ptr::from_ref(#field_variable);
                                    unsafe { ptr.read() }
                                },
                            }
                        })
                        .collect();

                    quote! {
                        #strukt_module::#variant_ident {
                            #(#field_setters)*
                        }
                    }
                })
                .collect();

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
                                let strukt = #strukt_constructors;
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

        Data::Struct(_) | Data::Union(_) => Err(PackedError::NotAnEnum),
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
