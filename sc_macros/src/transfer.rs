use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::quote;

use syn::{
  parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Field, Fields, Ident, Token,
  Variant,
};

pub fn transfer(input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(input as DeriveInput);

  match args.data {
    Data::Enum(en) => t_enum(args.ident, en.variants),
    Data::Struct(s) => t_struct(args.ident, s.fields),
    Data::Union(_) => unimplemented!("unions are not supported!"),
  }
}

fn t_enum(name: Ident, variants: Punctuated<Variant, Token![,]>) -> TokenStream {
  let id: Vec<_> =
    variants.iter().enumerate().map(|(i, _)| Literal::u32_unsuffixed(i as u32)).collect();
  let variant: Vec<_> = variants.iter().enumerate().map(|(_, v)| v.ident.clone()).collect();
  let field: Vec<Vec<_>> = variants
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
          )*
          v => panic!("unknown packet id {}", v),
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

fn t_struct(name: Ident, fields: Fields) -> TokenStream {
  let out = match fields {
    Fields::Named(f) => {
      let field = f.named.iter().map(|v| &v.ident).collect::<Vec<_>>();
      quote! {
        impl #name {
          pub fn read(m: &mut sc_transfer::MessageReader) -> Result<Self, sc_transfer::ReadError> {
            Ok(Self {
              #(
                #field: m.read()?,
              )*
            })
          }
          pub fn write(&self, m: &mut sc_transfer::MessageWriter) -> Result<(), sc_transfer::WriteError> {
            #(
              m.write(&self.#field)?;
            )*
            Ok(())
          }
        }
      }
    }
    _ => unimplemented!("fields: {:?}", fields),
  };
  out.into()
}
