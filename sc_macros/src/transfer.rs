use proc_macro::TokenStream;
use proc_macro2::{Literal, Span};
use quote::quote;

use syn::{
  parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Fields, Ident, Token, Variant,
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
  let variant_read: Vec<_> = variants
    .iter()
    .enumerate()
    .map(|(id, v)| {
      let variant = &v.ident;
      let id = Literal::u32_unsuffixed(id as u32);
      match &v.fields {
        Fields::Named(f) => {
          let field = f.named.iter().map(|v| &v.ident).collect::<Vec<_>>();
          quote! {
            #id => Self::#variant { #( #field: m.read()? ),* },
          }
        }
        Fields::Unnamed(f) => {
          let reader = f.unnamed.iter().map(|_| quote!(m.read()?));
          quote! {
            #id => Self::#variant(#( #reader ),*),
          }
        }
        Fields::Unit => quote! {
          #id => Self::#variant,
        },
      }
    })
    .collect();
  let variant_write: Vec<_> = variants
    .iter()
    .enumerate()
    .map(|(id, v)| {
      let variant = &v.ident;
      let id = Literal::u32_unsuffixed(id as u32);
      match &v.fields {
        Fields::Named(f) => {
          let field = f.named.iter().map(|v| &v.ident).collect::<Vec<_>>();
          quote! {
            Self::#variant { #( #field ),* } => {
              m.write_u32(#id)?;
              #( m.write(#field)?; )*
            }
          }
        }
        Fields::Unnamed(f) => {
          let field = f
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, _)| Ident::new(&format!("v{}", i), Span::call_site()))
            .collect::<Vec<_>>();
          quote! {
            Self::#variant(#( #field ),*) => {
              m.write_u32(#id)?;
              #( m.write(#field)?; )*
            }
          }
        }
        Fields::Unit => quote! {
          Self::#variant => {
            m.write_u32(#id)?;
          }
        },
      }
    })
    .collect();

  let out = quote! {
    impl sc_transfer::MessageRead for #name {
      fn read(m: &mut sc_transfer::MessageReader) -> Result<Self, sc_transfer::ReadError> {
        Ok(match m.read_u32()? {
          #(
            #variant_read
          )*
          v => panic!("unknown packet id {}", v),
        })
      }
    }
    impl sc_transfer::MessageWrite for #name {
      fn write(&self, m: &mut sc_transfer::MessageWriter) -> Result<(), sc_transfer::WriteError> {
        match self {
          #(
            #variant_write
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
        impl sc_transfer::MessageRead for #name {
          fn read(m: &mut sc_transfer::MessageReader) -> Result<Self, sc_transfer::ReadError> {
            Ok(Self {
              #(
                #field: m.read()?,
              )*
            })
          }
        }
        impl sc_transfer::MessageWrite for #name {
          fn write(&self, m: &mut sc_transfer::MessageWriter) -> Result<(), sc_transfer::WriteError> {
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
