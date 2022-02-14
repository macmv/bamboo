use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{
  parse::{ParseStream, Parser},
  parse_macro_input, Attribute, Fields, Item, LitInt, Token,
};

pub fn transfer(input: TokenStream) -> TokenStream {
  let mut args = parse_macro_input!(input as Item);

  let block;
  match &mut args {
    Item::Struct(s) => {
      let ty = &s.ident;
      let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();
      let setter = create_setter(&mut s.fields);

      block = quote! {
        impl #impl_generics sc_transfer::MessageRead<'_> for #ty #ty_generics #where_clause {
          fn read(&self, m: &mut sc_transfer::MessageReader) -> Result<Self, sc_transfer::ReadError> {
            m.read_struct::<Self>()
          }
        }

        impl #impl_generics sc_transfer::StructRead for #ty #ty_generics #where_clause {
          fn read_struct(mut m: sc_transfer::StructReader) -> Result<Self, sc_transfer::ReadError> {
            Ok(Self #setter)
          }
        }
      };
    }
    Item::Enum(e) => {
      let ty = &e.ident;
      let (impl_generics, ty_generics, where_clause) = e.generics.split_for_impl();
      let mut variants = vec![];
      let mut ids = vec![];
      let mut setters = vec![];
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
        variants.push(&v.ident);
        ids.push(id);
        setters.push(create_setter(&mut v.fields));
      }

      block = quote! {
        impl #impl_generics sc_transfer::MessageRead<'_> for #ty #ty_generics #where_clause {
          fn read(&self, m: &mut sc_transfer::MessageReader) -> Result<Self, sc_transfer::ReadError> {
            m.read_enum::<Self>()
          }
        }

        impl #impl_generics sc_transfer::EnumRead for #ty #ty_generics #where_clause {
          fn read_enum(mut m: sc_transfer::EnumReader) -> Result<Self, sc_transfer::ReadError> {
            Ok(match m.variant() {
              #(
                #ids => Self::#variants #setters,
              )*
              _ => return Err(m.invalid_variant()),
            })
          }
        }
      };
    }
    _ => unimplemented!(),
  }

  let out = quote! {
    #args

    #block
  };

  out.into()
  /*
  match args {
    TransferInput::Enum(en) => t_enum(args.ident, args.generics, en.variants),
    TransferInput::Struct(s) => t_struct(args.ident, args.generics, s.fields),
  }
  */
}

fn parse_id(input: ParseStream) -> Result<u64, syn::Error> {
  let _: Token![=] = input.parse()?;
  let lit: LitInt = input.parse()?;
  lit.base10_parse()
}

fn find_id(attrs: &[Attribute]) -> Option<(usize, u64)> {
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

fn create_setter(f: &mut Fields) -> TokenStream2 {
  match f {
    Fields::Unit => quote!(),
    Fields::Unnamed(fields) => {
      let mut out = quote!();
      for (i, _) in fields.unnamed.iter().enumerate() {
        let i = i as u64;
        out.extend(quote!(m.read(#i)?,));
      }
      quote!((#out))
    }
    Fields::Named(fields) => {
      let mut ids = vec![];
      let mut names = vec![];
      for (i, f) in &mut fields.named.iter_mut().enumerate() {
        let id = match find_id(&f.attrs) {
          Some((idx, id)) => {
            f.attrs.remove(idx);
            id
          }
          None => i as u64,
        };
        names.push(f.ident.as_ref().unwrap());
        ids.push(id);
      }
      quote!({
        #(
          #names: m.read(#ids)?,
        )*
      })
    }
  }
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
