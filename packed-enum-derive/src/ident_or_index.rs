use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Ident, Index};

pub enum IdentOrIndex<'a> {
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
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            IdentOrIndex::Ident(ident) => ident.to_tokens(tokens),
            IdentOrIndex::Index(i) => i.to_tokens(tokens),
        }
    }
}
