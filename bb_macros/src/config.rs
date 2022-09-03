use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use syn::{parse_macro_input, punctuated::Punctuated, Fields, Item, Token};

fn error(value: Option<&Ident>, error: &str) -> TokenStream {
  match value {
    Some(v) => {
      let err = quote_spanned!(v.span() => compile_error!(#error));
      // Provide a dummy impl, in order to produce less errors.
      quote!(
        #err;
        impl crate::config::Config for #v {
          fn from_config(c: &crate::config::Value) -> Option<Self> { todo!() }
          fn name() -> String { todo!() }
        }
      )
    }
    None => {
      quote!(compile_error!(#error);)
    }
  }
  .into()
}

pub fn config(input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(input as Item);

  let out = match args {
    Item::Struct(s) => {
      let name = &s.ident;
      let fields = match s.fields {
        Fields::Named(named) => named
          .named
          .into_iter()
          .map(|field| {
            let name = field.ident.unwrap();
            let name_str = name.to_string();
            let ty = field.ty;
            quote!(
              #name: <#ty as crate::config::TomlValue>::from_toml(t.get(#name_str)?)?
            )
          })
          .collect::<Punctuated<TokenStream2, Token![,]>>(),
        _ => return error(Some(name), "expected named fields"),
      };
      quote!(
        impl crate::config::TomlValue for #name {
          fn from_toml(value: &crate::config::Value) -> Option<Self> {
            match value {
              crate::config::Value::Table(t) => Some(Self {
                #fields
              }),
              _ => None,
            }
          }
          fn name() -> String { todo!() }
        }
      )
    }
    _ => return error(None, "expected a struct or enum"),
  };

  out.into()
}
