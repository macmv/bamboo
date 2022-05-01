use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Expr, Fields, GenericArgument, ItemEnum, Lit, PathArguments, Type};

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
  let mut size = vec![];
  for v in input.variants.iter() {
    match &v.fields {
      Fields::Named(fields) => {
        let mut index = 0;
        let mut starts = vec![];
        let mut ends = vec![];
        for field in &fields.named {
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
      pub fn access_mut<R>(&mut self, index: u32, f: impl FnOnce(&mut Stack) -> R) -> Option<R> {
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
      pub fn size(&self) -> u32 {
        match self {
          #(
            Self::#variant { .. } => #size,
          )*
        }
      }
    }
  };
  // Will print the result of this proc macro
  let mut p =
    std::process::Command::new("rustfmt").stdin(std::process::Stdio::piped()).spawn().unwrap();
  std::io::Write::write_all(p.stdin.as_mut().unwrap(), out.to_string().as_bytes()).unwrap();
  p.wait_with_output().unwrap();

  out.into()
}
