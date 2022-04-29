#![allow(dead_code)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashMap;
use syn::{
  parse::{Parse, ParseStream, Result},
  parse_macro_input,
  punctuated::Punctuated,
  Expr, Ident, Token,
};

struct Behaviors {
  defs:     Vec<Def>,
  def_map:  HashMap<String, Vec<Ident>>,
  mappings: Vec<Mapping>,
}

struct Def {
  key:    KeyDef,
  eq:     Token![=],
  values: Punctuated<Ident, Token![,]>,
  semi:   Token![;],
}
struct Mapping {
  keys:  Punctuated<MapKey, Token![|]>,
  arrow: Token![=>],
  value: Expr,
  semi:  Token![;],
}
#[derive(Clone)]
struct MapKey {
  sections: Vec<KeySection>,
}

#[derive(Clone)]
enum KeySection {
  Def(KeyDef),
  Lit(Ident),
}

#[derive(Clone)]
struct KeyDef {
  star:  Token![*],
  key:   Ident,
  star2: Token![*],
}

impl Parse for Behaviors {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut defs: Vec<Def> = vec![];
    let mut mappings = vec![];
    loop {
      if input.is_empty() {
        break Ok(Behaviors {
          def_map: defs
            .iter()
            .map(|def| (def.key.key.to_string(), def.values.iter().cloned().collect()))
            .collect(),
          defs,
          mappings,
        });
      }
      let mut keys: Punctuated<MapKey, Token![|]> = Punctuated::parse_separated_nonempty(input)?;
      let look = input.lookahead1();
      if look.peek(Token![=>]) {
        mappings.push(Mapping {
          keys,
          arrow: input.parse()?,
          value: input.parse()?,
          semi: input.parse()?,
        });
      } else if look.peek(Token![=]) {
        defs.push(Def {
          key:    match keys.pop().unwrap().value().sections.first().unwrap().clone() {
            KeySection::Def(d) => d,
            _ => return Err(look.error()),
          },
          eq:     input.parse()?,
          values: Punctuated::parse_separated_nonempty(input)?,
          semi:   input.parse()?,
        });
      } else {
        return Err(look.error());
      }
    }
  }
}

impl Parse for MapKey {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut sections = vec![];
    loop {
      let look = input.lookahead1();
      if look.peek(Token![*]) {
        sections.push(KeySection::Def(input.parse()?));
      } else if look.peek(Ident) {
        sections.push(KeySection::Lit(input.parse()?));
      } else {
        return Ok(MapKey { sections });
      }
    }
  }
}

impl Parse for KeyDef {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(KeyDef { star: input.parse()?, key: input.parse()?, star2: input.parse()? })
  }
}

impl Behaviors {
  pub fn expand(self) -> TokenStream {
    let mut out = vec![];
    for mapping in self.mappings {
      let expr = mapping.value;
      for key in mapping.keys {
        let mut list = vec![];
        key.all_keys(&self.def_map, &mut list);
        for key in list {
          out.push(quote!(out.set(Kind::#key, Box::new(#expr))));
        }
      }
    }
    quote! {
      #(
        #out;
      )*
    }
    .into()
  }
}

impl MapKey {
  fn all_keys(self, defs: &HashMap<String, Vec<Ident>>, out: &mut Vec<Ident>) {
    self.all_keys_inner("".into(), defs, out);
  }
  fn all_keys_inner(
    mut self,
    prefix: String,
    defs: &HashMap<String, Vec<Ident>>,
    out: &mut Vec<Ident>,
  ) {
    if self.sections.is_empty() {
      out.push(Ident::new(&prefix, Span::call_site()));
      return;
    }
    let first = self.sections.remove(0);
    match first {
      KeySection::Lit(lit) => self.all_keys_inner(prefix + &lit.to_string(), defs, out),
      KeySection::Def(def) => {
        for val in &defs[&def.key.to_string()] {
          self.clone().all_keys_inner(prefix.clone() + &val.to_string(), defs, out)
        }
      }
    }
  }
}

pub fn behavior(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as Behaviors);
  input.expand()
}
