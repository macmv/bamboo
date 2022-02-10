use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
  parse::{ParseStream, Parser, Result},
  parse_macro_input,
  spanned::Spanned,
  Attribute, Item, LitInt, Token,
};

pub fn transfer(input: TokenStream) -> TokenStream {
  let mut args = parse_macro_input!(input as Item);

  match &mut args {
    Item::Struct(s) => {
      let mut ids = vec![];
      for f in &mut s.fields {
        let (idx, id) = match find_id(&f.attrs) {
          Some(v) => v,
          None => {
            return quote_spanned!(
              f.vis.span().join(f.ty.span()).unwrap() =>
              compile_error!("all fields must list an id with #[id = 0]");
            )
            .into()
          }
        };
        f.attrs.remove(idx);
        ids.push(id);
      }
    }
    Item::Enum(e) => {
      let mut ids = vec![];
      for v in &mut e.variants {
        let (idx, id) = match find_id(&v.attrs) {
          Some(v) => v,
          None => {
            return quote_spanned!(
              v.ident.span() =>
              compile_error!("all fields must list an id with #[id = 0]");
            )
            .into()
          }
        };
        v.attrs.remove(idx);
        ids.push(id);
        for f in &mut v.fields {
          f.attrs.clear();
        }
      }
    }
    _ => unimplemented!(),
  }

  let out = quote!(#args);

  out.into()
  /*
  match args {
    TransferInput::Enum(en) => t_enum(args.ident, args.generics, en.variants),
    TransferInput::Struct(s) => t_struct(args.ident, args.generics, s.fields),
  }
  */
}

fn parse_id(input: ParseStream) -> Result<u32> {
  let _: Token![=] = input.parse()?;
  let lit: LitInt = input.parse()?;
  lit.base10_parse()
}

fn find_id(attrs: &[Attribute]) -> Option<(usize, u32)> {
  for (i, a) in attrs.iter().enumerate() {
    if a.path.get_ident().map(|i| i == "id").unwrap_or(false) {
      let id = match parse_id.parse2(a.tokens.clone()) {
        Ok(v) => v,
        Err(_) => continue,
      };
      return Some((i, id));
    }
  }
  None
}

/*
fn t_enum(
  name: Ident,
  generics: Generics,
  variants: Punctuated<Variant, Token![,]>,
) -> TokenStream {
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

  let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

  let out = quote! {
    impl #impl_generics sc_transfer::MessageRead for #name #ty_generics #where_clause {
      fn read(m: &mut sc_transfer::MessageReader) -> Result<Self, sc_transfer::ReadError> {
        Ok(match m.read_u32()? {
          #(
            #variant_read
          )*
          v => panic!("unknown enum id {}", v),
        })
      }
    }
    impl #impl_generics sc_transfer::MessageWrite for #name #ty_generics #where_clause {
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

fn t_struct(name: Ident, generics: Generics, fields: Fields) -> TokenStream {
  let out = match fields {
    Fields::Named(f) => {
      let field = f.named.iter().map(|v| &v.ident).collect::<Vec<_>>();
      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
      quote! {
        impl #impl_generics sc_transfer::MessageRead for #name #ty_generics #where_clause {
          fn read(m: &mut sc_transfer::MessageReader) -> Result<Self, sc_transfer::ReadError> {
            Ok(Self {
              #(
                #field: m.read()?,
              )*
            })
          }
        }
        impl #impl_generics sc_transfer::MessageWrite for #name #ty_generics #where_clause {
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
*/
