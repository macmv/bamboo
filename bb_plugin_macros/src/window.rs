use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
  parse_macro_input, Expr, Fields, GenericArgument, ItemEnum, Lit, LitStr, PathArguments, Type,
};

#[allow(clippy::collapsible_match)]
pub fn window(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ItemEnum);
  let ty = &input.ident;
  let variant: Vec<_> = input.variants.iter().map(|v| &v.ident).collect();
  let field: Vec<Vec<_>> = input
    .variants
    .iter()
    .map(|v| match &v.fields {
      Fields::Named(fields) => fields.named.iter().map(|f| &f.ident).collect(),
      _ => panic!(),
    })
    .collect();
  let mut field_start = vec![];
  let mut field_end = vec![];
  let mut field_non_outputs = vec![];
  let mut names = vec![];
  let mut size = vec![];
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
    match &v.fields {
      Fields::Named(fields) => {
        let mut index = 0;
        let mut starts = vec![];
        let mut ends = vec![];
        let mut non_outputs = vec![];
        for field in &fields.named {
          let mut output = false;
          for attr in &field.attrs {
            if attr.path.get_ident().map(|i| i == "output").unwrap_or(false) {
              output = true;
            } else if attr.path.get_ident().map(|i| i == "filter").unwrap_or(false) {
              // TODO: Handle
            }
          }
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
                    _ => panic!(),
                  },
                  _ => panic!(),
                },
                _ => panic!(),
              },
              _ => panic!(),
            },
            _ => panic!(),
          }
        }
        field_start.push(starts);
        field_end.push(ends);
        size.push(index);
        field_non_outputs.push(non_outputs);
      }
      _ => panic!(),
    }
  }
  let out = quote! {
    impl #ty {
      pub fn access<R>(&self, index: u32, f: impl FnOnce(&Stack) -> R) -> Option<R> {
        match self {
          #(
            Self::#variant { #(#field),* } => {
              match index {
                #(
                  #field_start..=#field_end => #field.lock().get(index - #field_start).map(|s| f(s)),
                )*
                _ => None,
              }
            }
          )*
        }
      }
      pub(crate) fn access_mut<R>(&mut self, index: u32, f: impl FnOnce(&mut Stack) -> R) -> Option<R> {
        match self {
          #(
            Self::#variant { #(#field),* } => {
              match index {
                #(
                  #field_start..=#field_end => #field.lock().get_mut(index - #field_start).map(|s| f(s)),
                )*
                _ => None,
              }
            }
          )*
        }
      }
      pub fn sync(&self, index: u32) {
        match self {
          #(
            Self::#variant { #(#field),* } => {
              match index {
                #(
                  #field_start..=#field_end => #field.lock().sync(index - #field_start),
                )*
                _ => panic!("cannot sync index out of bounds {}", index),
              }
            }
          )*
        }
      }
      pub fn open(&self, id: UUID, conn: &ConnSender) {
        match self {
          #(
            Self::#variant { #(#field),* } => {
              #(
                #field.lock().open(id, conn.clone());
              )*
            }
          )*
        }
      }
      pub fn close(&self, id: UUID) {
        match self {
          #(
            Self::#variant { #(#field),* } => {
              #(
                #field.lock().close(id);
              )*
            }
          )*
        }
      }
      pub fn add(&mut self, stack: &Stack) -> u8 {
        let mut stack = stack.clone();
        match self {
          #(
            Self::#variant { #(#field_non_outputs,)* .. } => {
              #(
                let amount = #field_non_outputs.lock().add(&stack);
                if amount == 0 {
                  return 0;
                }
                stack.set_amount(amount);
              )*
              stack.amount()
            }
          )*
        }
      }
      pub fn size(&self) -> u32 {
        match self {
          #(
            Self::#variant { .. } => #size,
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
