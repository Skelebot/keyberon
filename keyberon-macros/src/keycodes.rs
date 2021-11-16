use quote::quote;
use proc_macro_error::emit_error;
use proc_macro2::*;

pub fn punctuation_to_keycode(p: &Punct, out: &mut TokenStream) {
    match p.as_char() {
        // Normal punctuation
        '-' => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Minus), }),
        '=' => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Equal), }),
        ';' => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::SColon), }),
        ',' => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Comma), }),
        '.' => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Dot), }),
        '/' => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Slash), }),

        // Shifted punctuation
        '!' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb1]), }),
        '@' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb2]), }),
        '#' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb3]), }),
        '$' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb4]), }),
        '%' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb5]), }),
        '^' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb6]), }),
        '&' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb7]), }),
        '*' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb8]), }),
        '_' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Minus]), }),
        '+' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Equal]), }),
        '|' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Bslash]), }),
        '~' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Grave]), }),
        '<' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Comma]), }),
        '>' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Dot]), }),
        '?' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Slash]), }),
        ':' => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::SColon]), }),
        // Is this reachable?
        _ => emit_error!(p, "Punctuation could not be parsed as a keycode")
    }
}

pub fn literal_to_keycode(l: &Literal, out: &mut TokenStream) {
    //let repr = l.to_string();
    match l.to_string().as_str() {
        "1" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb1), }),
        "2" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb2), }),
        "3" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb3), }),
        "4" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb4), }),
        "5" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb5), }),
        "6" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb6), }),
        "7" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb7), }),
        "8" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb8), }),
        "9" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb9), }),
        "0" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Kb0), }),

        // Char literals; mostly punctuation which can't be properly tokenized alone
        r#"'\''"# => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Quote), }),
        r#"'\\'"# => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Bslash), }),
        // Shifted characters
        "'['" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::LBracket), }),
        "']'" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::RBracket), }),
        "'`'" => out.extend(quote! { keyberon::action::Action::KeyCode(keyberon::key_code::KeyCode::Grave), }),
        "'\"'" => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Quote]), }),
        "'('" => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb9]), }),
        "')'" => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Kb0]), }),
        "'{'" => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::LBracket]), }),
        "'}'" => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::RBracket]), }),
        "'_'" => out.extend(quote! { keyberon::action::Action::MultipleKeyCodes(&[keyberon::key_code::KeyCode::LShift, keyberon::key_code::KeyCode::Minus]), }),

        s if s.starts_with('\'') => emit_error!(l, "Literal could not be parsed as a keycode"; help = "Maybe try without quotes?"),

        s if s.starts_with('\"')  => {
            if s.len() == 3 {
                emit_error!(l, "Typing strings on key press is not yet supported"; help = "Did you mean to use apostrophes instead of quotes?");
            } else {
                emit_error!(l, "Typing strings on key press is not yet supported");
            }
        }
        _ => emit_error!(l, "Literal could not be parsed as a keycode")
    }
}