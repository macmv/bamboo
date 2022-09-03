use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use std::{fmt, fmt::Write};

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

struct VariantsTemplate<'a>(&'a [String]);

impl fmt::Display for VariantsTemplate<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "got invalid option '{{}}', valid options are")?;
    if self.0.len() == 1 {
      write!(f, " '{}'", self.0[0])
    } else if self.0.len() == 2 {
      write!(f, " '{}' or '{}'", self.0[0], self.0[1])
    } else {
      let has_too_many = self.0.len() > 20;
      let variants = if has_too_many { &self.0[..20] } else { &self.0[..] };
      for (i, variant) in variants.iter().enumerate() {
        if i == variants.len() - 1 {
          write!(f, " or")?;
        }
        write!(f, " '{variant}'")?;
        if i != variants.len() - 1 {
          write!(f, ",")?;
        }
      }
      if has_too_many {
        write!(f, " (skipping {} options)", self.0.len() - 20)?;
      }
      Ok(())
    }
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
                t.get(#name_str).ok_or(crate::config::ConfigError::other(
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
      let mut variant_strings = vec![];
      let variants = en
        .variants
        .iter()
        .map(|variant| match variant.fields {
          Fields::Unit => {
            let ident = &variant.ident;
            let ident_str = variant.ident.to_string().to_lowercase();
            variant_strings.push(ident_str.clone());
            Ok(quote!(#ident_str => Ok(Self::#ident)))
          }
          _ => Err(variant),
        })
        .collect::<Result<Punctuated<TokenStream2, Token![,]>, _>>();
      let name = &en.ident;
      match variants {
        Ok(variants) => {
          let error_template = VariantsTemplate(&variant_strings).to_string();
          quote!(
            impl crate::config::TomlValue for #name {
              fn from_toml(value: &crate::config::Value) -> crate::config::Result<Self> {
                match value {
                  crate::config::Value::String(s) => match s.as_str() {
                    #variants,
                    _ => Err(crate::config::ConfigError::other(format!(#error_template, s))),
                  },
                  _ => Err(crate::config::ConfigError::from_value::<Self>(value)),
                }
              }
              fn name() -> String { stringify!(#name).into() }
            }
          )
        }
        Err(e) => return error_at(name, e.ident.span(), "expected a unit variant"),
      }
    }
    _ => return error(None, "expected a struct or enum"),
  };

  out.into()
}
