use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{quote, quote_spanned};

use syn::{parse_macro_input, Fields, Item};

fn error(value: Option<&Ident>, error: &str) -> TokenStream {
  match value {
    Some(v) => {
      let err = quote_spanned!(v.span() => compile_error!(#error));
      // Provide a dummy impl, in order to produce less errors.
      quote!(
        #err;
        impl crate::config::Config for #v {
          fn from_config(c: &crate::config::Config) -> Option<Self> { todo!() }
          fn name() -> String { todo!() }
        }
      )
    }
    None => {
      quote!(compile_error!(#error))
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
        Fields::Named(named) => quote!(a: 3),
        _ => return error(Some(name), "expected named fields"),
      };
      quote!(
        impl crate::config::TomlValue for #name {
          fn from_toml(c: &toml::Value) -> Option<Self> {
            Some(Self {
              #fields
            })
          }
          fn name() -> String { todo!() }
        }
      )
    }
    _ => return error(None, "expected a struct or enum"),
  };

  out.into()
}
