use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal, TokenStream as TokenStream2};
use quote::quote;
use std::collections::HashMap;

use syn::{
  braced,
  parse::{Parse, ParseStream, Result},
  parse_macro_input,
  spanned::Spanned,
  Attribute, Error, Expr, Fields, ItemEnum, Lit, LitInt, Token,
};

struct KeyedArgs {
  keys: HashMap<String, Expr>,
}

#[derive(Debug)]
struct LookupArgs {
  min:   f64,
  max:   f64,
  steps: usize,
  ty:    Ident,
}

impl Parse for KeyedArgs {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut keys = HashMap::new();

    loop {
      if input.is_empty() {
        break;
      }

      let key: Ident = input.parse()?;
      let _sep: Token![:] = input.parse()?;
      let val: Expr = input.parse()?;
      keys.insert(key.to_string(), val);

      if input.is_empty() {
        break;
      }
      let _comma: Token![,] = input.parse()?;
    }

    Ok(KeyedArgs { keys })
  }
}

impl Parse for LookupArgs {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut args = KeyedArgs::parse(input)?.keys;

    let min = match args.remove("min").ok_or(input.error("expected a `min` argument"))? {
      Expr::Lit(lit) => match lit.lit {
        Lit::Float(f) => f.base10_parse::<f64>()?,
        v => return Err(Error::new(v.span(), "expected an f64")),
      },
      v => return Err(Error::new(v.span(), "expected an f64")),
    };
    let max = match args.remove("max").ok_or(input.error("expected a `max` argument"))? {
      Expr::Lit(lit) => match lit.lit {
        Lit::Float(f) => f.base10_parse::<f64>()?,
        v => return Err(Error::new(v.span(), "expected an f64")),
      },
      v => return Err(Error::new(v.span(), "expected an f64")),
    };
    let steps = match args.remove("steps").ok_or(input.error("expected a `steps` argument"))? {
      Expr::Lit(lit) => match lit.lit {
        Lit::Int(v) => v.base10_parse::<usize>()?,
        v => return Err(Error::new(v.span(), "expected a usize")),
      },
      v => return Err(Error::new(v.span(), "expected a usize")),
    };
    let ty = match args.remove("ty").ok_or(input.error("expected a `ty` argument"))? {
      Expr::Path(path) => path.path.segments.first().unwrap().ident.clone(),
      v => return Err(Error::new(v.span(), "expected a type name (like f64)")),
    };

    Ok(LookupArgs { min, max, steps, ty })
  }
}

impl LookupArgs {
  fn convert(&self, v: f64) -> Result<TokenStream2> {
    let res = v.cos();
    if self.ty == "f32" {
      let res = res as f32;
      Ok(quote!(#res))
    } else if self.ty == "f64" {
      Ok(quote!(#res))
    } else {
      Err(Error::new(self.ty.span(), "invalid type"))
    }
  }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn lookup_table(input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(input as LookupArgs);

  let mut out: Vec<TokenStream2> = vec![];

  for step in 0..args.steps {
    let percent = step as f64 / args.steps as f64;
    let val = ((args.max - args.min) * percent) + args.min;
    match args.convert(val) {
      Ok(v) => out.push(v),
      Err(e) => return e.into_compile_error().into(),
    }
  }

  let out = quote!([#(#out),*]);
  out.into()
}

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

#[proc_macro_attribute]
pub fn protocol_version(_args: TokenStream, input: TokenStream) -> TokenStream {
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

#[proc_macro_derive(Packet)]
pub fn packet(input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(input as ItemEnum);

  let name = args.ident;
  let id: Vec<_> =
    args.variants.iter().enumerate().map(|(i, _)| Literal::u32_unsuffixed(i as u32)).collect();
  let variant: Vec<_> = args.variants.iter().enumerate().map(|(_, v)| v.ident.clone()).collect();
  let field: Vec<Vec<_>> = args
    .variants
    .iter()
    .enumerate()
    .map(|(_, v)| match &v.fields {
      Fields::Named(n) => n.named.iter().map(|f| f.ident.as_ref().unwrap()).collect(),
      _ => panic!("must have struct variant for all packet variants"),
    })
    .collect();

  let out = quote! {
    impl #name {
      pub fn read(m: &mut sc_transfer::MessageReader) -> Result<Self, sc_transfer::ReadError> {
        Ok(match m.read_u32()? {
          #(
            #id => {
              Self::#variant {
                #(
                  #field: m.read()?,
                )*
              }
            }
            v => panic!("unknown packet id {}", v),
          )*
        })
      }
      pub fn write(&self, m: &mut sc_transfer::MessageWriter) -> Result<(), sc_transfer::WriteError> {
        match self {
          #(
            Self::#variant { #( #field ),* } => {
              m.write_u32(#id)?;
              #(
                m.write(#field)?;
              )*
            }
          )*
        }
        Ok(())
      }
    }
  };
  out.into()
}
