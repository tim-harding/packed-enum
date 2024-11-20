use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub enum VariantKind {
    Empty,
    Tuple(usize),
    Struct,
}

impl ToTokens for VariantKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
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
