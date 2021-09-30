extern crate proc_macro;
use proc_macro2::{Delimiter, Group, Punct, Spacing, TokenStream, TokenTree};
use proc_macro_error::{abort, emit_error};
use quote::quote;

use crate::keycodes::*;

pub fn parse_layout(input: TokenStream) -> TokenStream {
    let mut out = TokenStream::new();

    for t in input {
        match t {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                let layer = parse_layer(g.stream());
                out.extend(quote! {
                    [#layer],
                });
            }
            //TokenTree::Punct(p) if p.as_char() == '#' => {

            //},
            _ => abort!(t, "Invalid token, expected layer: {{ ... }}"),
        }
    }

    out
}

pub fn parse_layer(input: TokenStream) -> TokenStream {
    let mut out = TokenStream::new();

    for t in input {
        match t {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket => {
                let row = parse_row(g.stream());
                out.extend(quote! {
                    [#row],
                });
            }
            TokenTree::Punct(p) if p.as_char() == ',' => (),
            _ => abort!(t, "Invalid token, expected row: [ ... ]"),
        }
    }

    out
}

pub fn parse_row(input: TokenStream) -> TokenStream {
    let mut out = TokenStream::new();

    for t in input {
        match t {
            TokenTree::Ident(i) => match i.to_string().as_str() {
                "n" => out.extend(quote! { keyberon::action::Action::NoOp, }),
                "t" => out.extend(quote! { keyberon::action::Action::Trans, }),
                _ => out.extend(quote! {
                    keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::#i),
                }),
            },
            TokenTree::Punct(p) => punctuation_to_keycode(&p, &mut out),
            TokenTree::Literal(l) => literal_to_keycode(&l, &mut out),
            TokenTree::Group(g) => parse_group(&g, &mut out),
        }
    }

    out
}

pub fn parse_group(g: &Group, out: &mut TokenStream) {
    match g.delimiter() {
        // Handle empty groups
        Delimiter::Parenthesis if g.stream().is_empty() => {
            emit_error!(g, "Expected a layer number in layer switch"; help = "To create a parenthesis keycode, enclose it in apostrophes: '('")
        }
        Delimiter::Brace if g.stream().is_empty() => {
            emit_error!(g, "Expected an action - group cannot be empty"; help = "To create a brace keycode, enclose it in apostrophes: '{'")
        }
        Delimiter::Bracket if g.stream().is_empty() => {
            emit_error!(g, "Expected keycodes - keycode group cannot be empty"; help = "To create a bracket keycode, enclose it in apostrophes: '['")
        }

        // Momentary layer switch (Action::Layer)
        Delimiter::Parenthesis => {
            let tokens = g.stream();
            out.extend(quote! { keyberon::action::Action::Layer(#tokens), });
        }
        // Pass the expression unchanged (adding a comma after it)
        Delimiter::Brace => out.extend(g.stream().into_iter().chain(TokenStream::from(
            TokenTree::Punct(Punct::new(',', Spacing::Alone)),
        ))),
        // Multiple keycodes (Action::MultipleKeyCodes)
        Delimiter::Bracket => parse_keycode_group(g.stream(), out),

        // Is this reachable?
        Delimiter::None => emit_error!(g, "Unexpected group"),
    }
}

pub fn parse_keycode_group(input: TokenStream, out: &mut TokenStream) {
    let mut inner = TokenStream::new();
    for t in input {
        match t {
            TokenTree::Ident(i) => inner.extend(quote! {
                keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::#i),
            }),
            TokenTree::Punct(p) => punctuation_to_keycode(&p, &mut inner),
            TokenTree::Literal(l) => literal_to_keycode(&l, &mut inner),
            TokenTree::Group(g) => parse_group(&g, &mut inner),
        }
    }
    out.extend(quote! { keyberon::action::Action::MultipleActions(&[#inner]) });
}
