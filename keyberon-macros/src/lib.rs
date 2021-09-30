extern crate proc_macro;
use proc_macro_error::proc_macro_error;
use quote::quote;

mod keycodes;
mod parse;
use crate::parse::*;

#[proc_macro_error]
#[proc_macro]
pub fn layout(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = parse_layout(input.into());

    (quote! { [#parsed] }).into()
}

#[proc_macro_error]
#[proc_macro]
pub fn layer(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = parse_layer(input.into());

    (quote! { [#parsed] }).into()
}

#[proc_macro_error]
#[proc_macro]
pub fn row(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = parse_row(input.into());

    (quote! { [#parsed] }).into()
}