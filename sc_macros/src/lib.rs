use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_error::abort;
use quote::{quote, ToTokens};
use std::collections::HashMap;

use syn::{
  parse::{Parse, ParseStream, Result},
  parse_macro_input,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Colon, Comma},
  AttributeArgs, Error, Expr, FnArg, Lit,
};

struct KeyedArgs {
  keys: HashMap<String, KeyedArg>,
}

struct KeyedArg {
  key: Ident,
  sep: Colon,
  val: Expr,
}

#[derive(Debug)]
struct LookupArgs {
  min:   f64,
  max:   f64,
  steps: usize,
  ty:    Ident,
  func:  Ident,
}

impl Parse for KeyedArgs {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut keys = HashMap::new();

    loop {
      if input.is_empty() {
        break;
      }

      let key: Ident = input.parse()?;
      let sep: Colon = input.parse()?;
      let val: Expr = input.parse()?;
      keys.insert(key.to_string(), KeyedArg { key, sep, val });

      if input.is_empty() {
        break;
      }
      let _comma: Comma = input.parse()?;
    }

    Ok(KeyedArgs { keys })
  }
}

impl Parse for LookupArgs {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut args = KeyedArgs::parse(input)?.keys;

    let min = match args.remove("min").ok_or(input.error("expected a `min` argument"))?.val {
      Expr::Lit(lit) => match lit.lit {
        Lit::Float(f) => f.base10_parse::<f64>()?,
        v => return Err(Error::new(v.span(), "expected an f64")),
      },
      v => return Err(Error::new(v.span(), "expected an f64")),
    };
    let max = match args.remove("max").ok_or(input.error("expected a `max` argument"))?.val {
      Expr::Lit(lit) => match lit.lit {
        Lit::Float(f) => f.base10_parse::<f64>()?,
        v => return Err(Error::new(v.span(), "expected an f64")),
      },
      v => return Err(Error::new(v.span(), "expected an f64")),
    };
    let steps = match args.remove("steps").ok_or(input.error("expected a `steps` argument"))?.val {
      Expr::Lit(lit) => match lit.lit {
        Lit::Int(v) => v.base10_parse::<usize>()?,
        v => return Err(Error::new(v.span(), "expected a usize")),
      },
      v => return Err(Error::new(v.span(), "expected a usize")),
    };
    let ty = match args.remove("ty").ok_or(input.error("expected a `ty` argument"))?.val {
      Expr::Path(path) => path.path.segments.first().unwrap().ident.clone(),
      v => return Err(Error::new(v.span(), "expected a type name (like f64)")),
    };
    let func = match args.remove("func").ok_or(input.error("expected a `func` argument"))?.val {
      Expr::Path(path) => path.path.segments.first().unwrap().ident.clone(),
      v => return Err(Error::new(v.span(), "expected a function name (like cos)")),
    };

    Ok(LookupArgs { min, max, steps, ty, func })
  }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn lookup_table(input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(input as LookupArgs);

  let mut out = vec![];

  for step in 0..args.steps {
    let percent = step as f64 / args.steps as f64;
    let val = ((args.max - args.min) * percent) + args.min;
    let num = val.cos();
    out.push(quote!(#num));
  }

  dbg!(&out);

  let out = quote!([#(#out),*]);
  out.into()
}
