use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_error::abort;
use quote::{quote, ToTokens};

use syn::{
  parse::{Parse, ParseStream, Result},
  parse_macro_input,
  punctuated::Punctuated,
  spanned::Spanned,
  token::Comma,
  AttributeArgs, Expr, FnArg,
};

#[derive(Debug)]
struct LookupArgs {
  args: Punctuated<Expr, Comma>,
}

impl Parse for LookupArgs {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut args = Punctuated::new();

    while !input.is_empty() {
      let first: Expr = input.parse()?;
      args.push_value(first);
      if input.is_empty() {
        break;
      }
      let punct = input.parse()?;
      args.push_punct(punct);
    }

    Ok(LookupArgs { args })
  }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn lookup_table(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as LookupArgs);
  dbg!(input);

  let out = quote! {};
  out.into()
}
