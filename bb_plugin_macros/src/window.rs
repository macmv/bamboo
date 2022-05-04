use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
  parse_macro_input, spanned::Spanned, Expr, GenericArgument, Ident, ItemEnum, ItemStruct, Lit,
  LitStr, PathArguments, Type,
};

#[allow(clippy::collapsible_match)]
pub fn window(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ItemStruct);
  let mut handler = None;
  for attr in input.attrs.iter() {
    if attr.path.get_ident().map(|i| i == "handler").unwrap_or(false) {
      match attr.parse_args::<Ident>() {
        Ok(ident) => {
          handler = Some(ident);
          break;
        }
        Err(err) => {
          let e = err.to_compile_error();
          return quote_spanned!(attr.path.span() => #e;).into();
        }
      }
    }
  }
  let handler = match handler {
    Some(h) => h,
    None => {
      return quote!(compile_error!("need handler attribute");).into();
    }
  };
  let mut field_names = vec![];
  let mut index = 0;
  let mut starts = vec![];
  let mut ends = vec![];
  let mut non_outputs = vec![];
  'fields: for field in &input.fields {
    let mut output = false;
    for attr in &field.attrs {
      if attr.path.get_ident().map(|i| i == "output").unwrap_or(false) {
        output = true;
      } else if attr.path.get_ident().map(|i| i == "not_inv").unwrap_or(false) {
        continue 'fields;
      } else if attr.path.get_ident().map(|i| i == "filter").unwrap_or(false) {
        // TODO: Handle
      }
    }
    field_names.push(&field.ident);
    if !output {
      non_outputs.push(&field.ident);
    }
    match &field.ty {
      Type::Path(p) => match &p.path.segments.first().unwrap().arguments {
        PathArguments::AngleBracketed(args) => match args.args.first().unwrap() {
          GenericArgument::Const(e) => match e {
            Expr::Lit(lit) => match &lit.lit {
              Lit::Int(int) => {
                let size = int.base10_parse::<u32>().unwrap();
                starts.push(index);
                ends.push(index + size - 1);
                index += size;
              }
              _ => {}
            },
            _ => {}
          },
          _ => {}
        },
        _ => {}
      },
      _ => {}
    }
  }

  let ty = &input.ident;
  let size = index;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let out = quote! {
    impl #impl_generics WindowData for #ty #ty_generics #where_clause {
      fn access<R>(&self, index: u32, f: impl FnOnce(&Stack) -> R) -> Option<R> {
        match index {
          #(
            #starts..=#ends => self.#field_names.lock().get(index - #starts).map(|s| f(s)),
          )*
          _ => None,
        }
      }
      fn access_mut<R>(&mut self, index: u32, f: impl FnOnce(&mut Stack) -> R) -> Option<R> {
        let ret = match index {
          #(
            #starts..=#ends => self.#field_names.lock().get_mut(index - #starts).map(|s| f(s)),
          )*
          _ => None,
        };
        #handler.on_update(Some(index), self);
        ret
      }
      fn sync(&self, index: u32) {
        match index {
          #(
            #starts..=#ends => self.#field_names.lock().sync(index - #starts),
          )*
          _ => panic!("cannot sync index out of bounds {}", index),
        }
      }
      fn open(&self, id: UUID, conn: &ConnSender) {
        #(
          self.#field_names.lock().open(id, conn.clone());
        )*
      }
      fn close(&self, id: UUID) {
        #(
          self.#field_names.lock().close(id);
        )*
      }
      fn add(&mut self, mut stack: Stack) -> u8 {
        #(
          let amount = self.#non_outputs.lock().add(&stack);
          if amount == 0 {
            #handler.on_update(None, self);
            return 0;
          }
          stack.set_amount(amount);
        )*
        #handler.on_update(None, self);
        stack.amount()
      }
      fn size(&self) -> u32 {
        #size
      }
    }
  };
  // Will print the result of this proc macro
  /*
  let mut p =
    std::process::Command::new("rustfmt").stdin(std::process::Stdio::piped()).spawn().unwrap();
  std::io::Write::write_all(p.stdin.as_mut().unwrap(), out.to_string().as_bytes()).unwrap();
  p.wait_with_output().unwrap();
  */

  out.into()
}

#[allow(clippy::collapsible_match)]
pub fn window_enum(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ItemEnum);
  let ty = &input.ident;
  let variant: Vec<_> = input.variants.iter().map(|v| &v.ident).collect();
  let mut names = vec![];
  for v in input.variants.iter() {
    let mut found_name = false;
    for attr in &v.attrs {
      if attr.path.get_ident().map(|i| i == "name").unwrap_or(false) {
        match attr.parse_args::<LitStr>() {
          Ok(lit) => {
            names.push(lit);
            found_name = true;
            break;
          }
          Err(err) => {
            let e = err.to_compile_error();
            return quote_spanned!(v.ident.span() => #e;).into();
          }
        }
      }
    }
    if !found_name {
      return quote_spanned!(v.ident.span() => compile_error!("requires #[name] attribute");)
        .into();
    }
  }
  let out = quote! {
    impl #ty {
      pub fn access<R>(&self, index: u32, f: impl FnOnce(&Stack) -> R) -> Option<R> {
        match self {
          #(
            Self::#variant(win) => win.access(index, f),
          )*
        }
      }
      pub(crate) fn access_mut<R>(&mut self, index: u32, f: impl FnOnce(&mut Stack) -> R) -> Option<R> {
        match self {
          #(
            Self::#variant(win) => win.access_mut(index, f),
          )*
        }
      }
      pub fn sync(&self, index: u32) {
        match self {
          #(
            Self::#variant(win) => win.sync(index),
          )*
        }
      }
      pub fn open(&self, id: UUID, conn: &ConnSender) {
        match self {
          #(
            Self::#variant(win) => win.open(id, conn),
          )*
        }
      }
      pub fn close(&self, id: UUID) {
        match self {
          #(
            Self::#variant(win) => win.close(id),
          )*
        }
      }
      pub fn add(&mut self, stack: &Stack) -> u8 {
        let mut stack = stack.clone();
        match self {
          #(
            Self::#variant(win) => win.add(stack),
          )*
        }
      }
      pub fn size(&self) -> u32 {
        match self {
          #(
            Self::#variant(win) => win.size(),
          )*
        }
      }
      pub fn ty(&self) -> &'static str {
        match self {
          #(
            Self::#variant { .. } => #names,
          )*
        }
      }
    }
  };
  // Will print the result of this proc macro
  /*
  let mut p =
    std::process::Command::new("rustfmt").stdin(std::process::Stdio::piped()).spawn().unwrap();
  std::io::Write::write_all(p.stdin.as_mut().unwrap(), out.to_string().as_bytes()).unwrap();
  p.wait_with_output().unwrap();
  */

  out.into()
}
