use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use syn::{parse_macro_input, punctuated::Punctuated, Fields, Item, Token};

fn error_at(v: &Ident, pos: Span, error: &str) -> TokenStream {
  let err = quote_spanned!(pos => compile_error!(#error));
  // Provide a dummy impl, in order to produce less errors.
  quote!(
    #err;
    impl crate::config::TomlValue for #v {
      fn from_toml(c: &crate::config::Value) -> crate::config::Result<Self> { todo!() }
      fn name() -> String { todo!() }
    }
  )
  .into()
}
fn error(value: Option<&Ident>, error: &str) -> TokenStream {
  match value {
    Some(v) => error_at(v, v.span(), error),
    None => quote!(compile_error!(#error);).into(),
  }
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
              #name: <#ty as crate::config::TomlValue>::from_toml(
                t.get(#name_str).ok_or(crate::config::ConfigError::new(
                  [].into_iter(),
                  concat!("missing field ", #name_str).to_string(),
                ))?)?
            )
          })
          .collect::<Punctuated<TokenStream2, Token![,]>>(),
        _ => return error(Some(name), "expected named fields"),
      };
      quote!(
        impl crate::config::TomlValue for #name {
          fn from_toml(value: &crate::config::Value) -> crate::config::Result<Self> {
            match value {
              crate::config::Value::Table(t) => Ok(Self {
                #fields
              }),
              _ => Err(crate::config::ConfigError::from_value::<Self>(value)),
            }
          }
          fn name() -> String { stringify!(#name).into() }
        }
      )
    }
    Item::Enum(en) => {
      let variants = en
        .variants
        .iter()
        .map(|variant| match variant.fields {
          Fields::Unit => {
            let ident = &variant.ident;
            let ident_str = variant.ident.to_string().to_lowercase();
            Ok(quote!(#ident_str => Ok(Self::#ident)))
          }
          _ => Err(variant),
        })
        .collect::<Result<Punctuated<TokenStream2, Token![,]>, _>>();
      let name = &en.ident;
      match variants {
        Ok(variants) => quote!(
          impl crate::config::TomlValue for #name {
            fn from_toml(value: &crate::config::Value) -> crate::config::Result<Self> {
              match value {
                crate::config::Value::String(s) => match s.as_str() {
                  #variants,
                  _ => Err(crate::config::ConfigError::new([].into_iter(), format!("invalid variant {}", s))),
                },
                _ => Err(crate::config::ConfigError::from_value::<Self>(value)),
              }
            }
            fn name() -> String { stringify!(#name).into() }
          }
        ),
        Err(e) => return error_at(name, e.ident.span(), "expected a unit variant"),
      }
    }
    _ => return error(None, "expected a struct or enum"),
  };

  out.into()
}
