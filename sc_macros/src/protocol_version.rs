use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal};
use quote::quote;

use syn::{
  braced,
  parse::{Parse, ParseStream, Result},
  parse_macro_input, Attribute, LitInt, Token,
};

struct ProtocolVersionArgs {
  attrs:    Vec<Attribute>,
  name:     Ident,
  versions: Vec<(Ident, u32, ProtocolVersion)>,
}

struct ProtocolVersion {
  maj: u32,
  min: u32,
}

impl Parse for ProtocolVersionArgs {
  fn parse(input: ParseStream) -> Result<Self> {
    let attrs = Attribute::parse_outer(input)?;
    let mut versions = vec![];

    let _pub: Token![pub] = input.parse()?;
    let _enum: Token![enum] = input.parse()?;
    let name: Ident = input.parse()?;
    let content;
    let _open = braced!(content in input);
    loop {
      if content.is_empty() {
        break;
      }

      let key: Ident = content.parse()?;
      let _sep: Token![=] = content.parse()?;
      let protocol: LitInt = content.parse()?;
      let ver = ProtocolVersion {
        maj: key.to_string().split('_').nth(1).unwrap().parse().unwrap(),
        min: key.to_string().split('_').nth(2).map(|s| s.parse().unwrap()).unwrap_or(0),
      };
      versions.push((key, protocol.base10_parse()?, ver));

      if content.is_empty() {
        break;
      }
      let _comma: Token![,] = content.parse()?;
    }

    Ok(ProtocolVersionArgs { attrs, name, versions })
  }
}
pub fn protocol_version(input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(input as ProtocolVersionArgs);

  let attrs = &args.attrs;
  let name = &args.name;
  let key = &args.versions.iter().map(|(key, _, _)| key).collect::<Vec<_>>();
  // Need unsuffixed for the enum definition
  let val =
    &args.versions.iter().map(|(_, val, _)| Literal::u32_unsuffixed(*val)).collect::<Vec<_>>();
  let maj = &args.versions.iter().map(|(_, _, ver)| ver.maj).collect::<Vec<_>>();
  let min = &args.versions.iter().map(|(_, _, ver)| ver.min).collect::<Vec<_>>();

  let out = quote! {
    #(#attrs)*
    pub enum #name {
      Invalid = 0,
      #(
        #key = #val,
      )*
    }

    impl #name {
      pub fn maj(&self) -> Option<u32> {
        Some(match self {
          Self::Invalid => return None,
          #(
            Self::#key => #maj,
          )*
        })
      }
      pub fn min(&self) -> Option<u32> {
        Some(match self {
          Self::Invalid => return None,
          #(
            Self::#key => #min,
          )*
        })
      }
    }
  };
  out.into()
}
