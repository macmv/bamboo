use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, Expr, Fields, Item, Token};

fn error_at(v: &Ident, pos: Span, error: &str) -> TokenStream {
  let err = quote_spanned!(pos => compile_error!(#error));
  // Provide a dummy impl, in order to produce less errors.
  quote!(
    #err;
    impl std::default::Default for #v {
      fn default() -> Self { todo!() }
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

struct DefaultValue(Expr);

impl Parse for DefaultValue {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let _: Token![=] = input.parse()?;
    Ok(DefaultValue(input.parse()?))
  }
}

pub fn default(input: TokenStream) -> TokenStream {
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
            for attr in &field.attrs {
              if attr.path.get_ident().map(|i| i == "default").unwrap_or(false) {
                let value = syn::parse::<DefaultValue>(attr.tokens.clone().into()).unwrap().0;
                return quote!(#name: #value);
              }
            }
            let ty = field.ty;
            quote!(#name: <#ty as std::default::Default>::default())
          })
          .collect::<Punctuated<TokenStream2, Token![,]>>(),
        _ => return error(Some(name), "expected named fields"),
      };
      quote!(
        impl std::default::Default for #name {
          fn default() -> Self {
            Self {
              #fields
            }
          }
        }
      )
    }
    Item::Enum(en) => {
      /*
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
          quote!(
            impl std::default::Default for #name {
              fn default() -> Self {
                Self::#default_variant
              }
            }
          )
        }
        Err(e) => return error_at(name, e.ident.span(), "expected a unit variant"),
      }
      */
      return error(None, "expected a struct or enum");
    }
    _ => return error(None, "expected a struct or enum"),
  };

  out.into()
}
