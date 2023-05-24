use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use std::fmt;

use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, Fields, Item, Token};

fn error_at(v: &Ident, pos: Span, error: &str) -> TokenStream {
  let err = quote_spanned!(pos => compile_error!(#error));
  // Provide a dummy impl, in order to produce less errors.
  quote!(
    #err;
    impl bb_common::config::TomlValue for #v {
      fn from_toml(c: &bb_common::config::Value) -> bb_common::config::Result<Self> { todo!() }
      fn to_toml(&self) -> bb_common::config::Value { todo!() }
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
    write!(f, "got invalid option \"{{}}\", valid options are")?;
    if self.0.len() == 1 {
      write!(f, " \"{}\"", self.0[0])
    } else if self.0.len() == 2 {
      write!(f, " \"{}\" or \"{}\"", self.0[0], self.0[1])
    } else {
      let has_too_many = self.0.len() > 20;
      let variants = if has_too_many { &self.0[..20] } else { self.0 };
      for (i, variant) in variants.iter().enumerate() {
        if i == variants.len() - 1 {
          write!(f, " or")?;
        }
        write!(f, " \"{variant}\"")?;
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
      let mut to_toml_pairs = vec![];
      let fields = match s.fields {
        Fields::Named(named) => named
          .named
          .into_iter()
          .map(|field| {
            let name = field.ident.unwrap();
            let name_str = name.to_string().replace('_', "-");
            let ty = field.ty;
            let comments = field.attrs.iter().flat_map(|attr| {
              if attr.path.get_ident().map(|v| v == "doc").unwrap_or(false) {
                let comment = syn::parse2::<DocComment>(attr.tokens.clone()).unwrap().0;
                let comment = comment.trim();
                Some(quote!(.with_comment(#comment)))
              } else {
                None
              }
            });
            to_toml_pairs.push(quote!(
              #name_str.to_string() => bb_common::config::TomlValue::to_toml(&self.#name) #( #comments )*
            ));
            quote!(
              #name: match t.get(#name_str) {
                Some(v) => <#ty as bb_common::config::TomlValue>::from_toml(v).map_err(|e| e.prepend(#name_str))?,
                None    => def.#name,
              }
            )
          })
          .collect::<Punctuated<TokenStream2, Token![,]>>(),
        _ => return error(Some(name), "expected named fields"),
      };
      quote!(
        impl bb_common::config::TomlValue for #name {
          fn from_toml(value: &bb_common::config::Value) -> bb_common::config::Result<Self> {
            let def = <Self as std::default::Default>::default();
            if let Some(t) = value.as_table() {
              Ok(Self {
                #fields
              })
            } else {
              Err(bb_common::config::ConfigError::from_value::<Self>(value))
            }
          }
          fn to_toml(&self) -> bb_common::config::Value {
            bb_common::config::Value::new(0, bb_common::indexmap! {
              #( #to_toml_pairs ),*
            })
          }
          fn name() -> String { stringify!(#name).into() }
        }
      )
    }
    Item::Enum(en) => {
      let mut variant_strings = vec![];
      let mut to_toml_match = vec![];
      let variants = en
        .variants
        .iter()
        .map(|variant| match variant.fields {
          Fields::Unit => {
            let ident = &variant.ident;
            let ident_str = variant.ident.to_string().to_lowercase();
            variant_strings.push(ident_str.clone());
            to_toml_match.push(quote!(Self::#ident => #ident_str));
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
            impl bb_common::config::TomlValue for #name {
              fn from_toml(value: &bb_common::config::Value) -> bb_common::config::Result<Self> {
                if let Some(s) = value.as_str() {
                  match s.as_str() {
                    #variants,
                    _ => Err(bb_common::config::ConfigError::other(format!(#error_template, s))),
                  }
                } else {
                  Err(bb_common::config::ConfigError::from_value::<Self>(value))
                }
              }
              fn to_toml(&self) -> bb_common::config::Value { bb_common::config::Value::new(0, match self {
                #( #to_toml_match ),*
              }) }
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

struct DocComment(String);

impl Parse for DocComment {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let _: Token![=] = input.parse()?;
    let s: syn::LitStr = input.parse()?;
    Ok(Self(s.value()))
  }
}
