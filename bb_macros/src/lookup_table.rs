use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use std::collections::HashMap;

use syn::{
  parse::{Parse, ParseStream, Result},
  parse_macro_input,
  spanned::Spanned,
  Error, Expr, Lit, Token,
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

    let min = match args.remove("min").ok_or_else(|| input.error("expected a `min` argument"))? {
      Expr::Lit(lit) => match lit.lit {
        Lit::Float(f) => f.base10_parse::<f64>()?,
        v => return Err(Error::new(v.span(), "expected an f64")),
      },
      v => return Err(Error::new(v.span(), "expected an f64")),
    };
    let max = match args.remove("max").ok_or_else(|| input.error("expected a `max` argument"))? {
      Expr::Lit(lit) => match lit.lit {
        Lit::Float(f) => f.base10_parse::<f64>()?,
        v => return Err(Error::new(v.span(), "expected an f64")),
      },
      v => return Err(Error::new(v.span(), "expected an f64")),
    };
    let steps =
      match args.remove("steps").ok_or_else(|| input.error("expected a `steps` argument"))? {
        Expr::Lit(lit) => match lit.lit {
          Lit::Int(v) => v.base10_parse::<usize>()?,
          v => return Err(Error::new(v.span(), "expected a usize")),
        },
        v => return Err(Error::new(v.span(), "expected a usize")),
      };
    let ty = match args.remove("ty").ok_or_else(|| input.error("expected a `ty` argument"))? {
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
