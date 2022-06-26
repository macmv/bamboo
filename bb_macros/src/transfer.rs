use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{
  parse::{ParseStream, Parser},
  parse_macro_input, Attribute, Fields, Ident, Item, LitInt, Token,
};

pub fn transfer(input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(input as Item);

  let block = match args {
    Item::Struct(s) => {
      let ty = &s.ident;
      let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();
      let (setter, write_len, writer) = create_setter(&s.fields, true);

      quote! {
        impl #impl_generics bb_transfer::MessageWrite for #ty #ty_generics #where_clause {
          fn write<__W: ::std::io::Write>(&self, m: &mut bb_transfer::MessageWriter<__W>) -> Result<(), bb_transfer::WriteError> {
            m.write_struct(#write_len, |m| #writer)
          }
        }

        impl #impl_generics bb_transfer::MessageRead<'_> for #ty #ty_generics #where_clause {
          fn read(m: &mut bb_transfer::MessageReader) -> Result<Self, bb_transfer::ReadError> {
            m.read_struct::<Self>()
          }
        }
        impl #impl_generics bb_transfer::StructRead<'_> for #ty #ty_generics #where_clause {
          fn read_struct(mut m: bb_transfer::StructReader) -> Result<Self, bb_transfer::ReadError> {
            Ok(Self #setter)
          }
        }
      }
    }
    Item::Enum(e) => {
      let ty = &e.ident;
      let (impl_generics, ty_generics, where_clause) = e.generics.split_for_impl();
      let mut idents = vec![];
      let mut variants = vec![];
      let mut ids = vec![];
      let mut readers = vec![];
      let mut writer_len = vec![];
      let mut writers = vec![];
      let mut empty_block = vec![];
      let mut variant_names = vec![];
      for v in &e.variants {
        idents.push(&v.ident);
        let (_, id) = match find_id(&v.attrs) {
          Some(v) => v,
          None => {
            return quote_spanned!(
              v.ident.span() =>
              compile_error!("all fields must list an id with #[id = 0]");
            )
            .into()
          }
        };
        variants.push(&v.ident);
        if let Some(idx) = ids.iter().position(|v| *v == id) {
          let err1 = quote_spanned!(
            idents[idx].span() => compile_error!("id first used here");
          );
          let err2 = quote_spanned!(
            v.ident.span() => compile_error!("duplicate id created here");
          );
          return quote!(#err1 #err2).into();
        }
        ids.push(id);
        let (read, write_len, write) = create_setter(&v.fields, false);
        readers.push(read);
        writer_len.push(write_len);
        writers.push(write);
        match &v.fields {
          Fields::Unit => {
            empty_block.push(quote!());
            variant_names.push(quote!());
          }
          Fields::Unnamed(fields) => {
            empty_block.push(quote!((..)));
            let names = fields
              .unnamed
              .iter()
              .enumerate()
              .map(|(i, _)| Ident::new(&format!("var_{}", i), Span::call_site()));
            variant_names.push(quote!(( #( #names ),* )));
          }
          Fields::Named(fields) => {
            empty_block.push(quote!({ .. }));
            let names = fields.named.iter().map(|f| f.ident.as_ref().unwrap());
            variant_names.push(quote!({ #( #names ),* }));
          }
        }
      }

      quote! {
        impl #impl_generics bb_transfer::MessageWrite for #ty #ty_generics #where_clause {
          fn write<__W: ::std::io::Write>(&self, m: &mut bb_transfer::MessageWriter<__W>) -> Result<(), bb_transfer::WriteError> {
            m.write_enum(match self {
              #(
                Self::#variants #empty_block => #ids,
              )*
            },
            match self {
              #(
                Self::#variants #empty_block => #writer_len,
              )*
            },
            |m| {
              match self {
                #(
                  Self::#variants #variant_names => #writers,
                )*
              }
            })
          }
        }
        impl #impl_generics bb_transfer::MessageRead<'_> for #ty #ty_generics #where_clause {
          fn read(m: &mut bb_transfer::MessageReader) -> Result<Self, bb_transfer::ReadError> {
            m.read_enum::<Self>()
          }
        }

        impl #impl_generics bb_transfer::EnumRead<'_> for #ty #ty_generics #where_clause {
          fn read_enum(mut m: bb_transfer::EnumReader) -> Result<Self, bb_transfer::ReadError> {
            Ok(match m.variant() {
              #(
                #ids => Self::#variants #readers,
              )*
              _ => return Err(m.invalid_variant()),
            })
          }
        }
      }
    }
    _ => unimplemented!(),
  };

  let out = quote! {
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
fn find_must_exist(attrs: &[Attribute]) -> Option<usize> {
  for (i, a) in attrs.iter().enumerate() {
    if a.path.get_ident().map(|i| i == "must_exist").unwrap_or(false) {
      return Some(i);
    }
  }
  None
}

/// Creates a reader and writer
fn create_setter(f: &Fields, has_self: bool) -> (TokenStream2, u64, TokenStream2) {
  match f {
    Fields::Unit => (quote!(), 0, quote!(Ok(()))),
    Fields::Unnamed(fields) => {
      let mut read = quote!();
      let mut write = quote!();
      for (i, f) in fields.unnamed.iter().enumerate() {
        let i = i as u64;
        let reader_name = match find_must_exist(&f.attrs) {
          Some(_) => quote!(must_read),
          None => quote!(read),
        };
        read.extend(quote!(m.#reader_name(#i)?,));
        if has_self {
          write.extend(quote!(m.write(&self.#i)?;));
        } else {
          let ident = Ident::new(&format!("var_{}", i), Span::call_site());
          write.extend(quote!(m.write(#ident)?;));
        }
      }
      (quote!((#read)), fields.unnamed.len() as u64, quote!({ #write Ok(()) }))
    }
    Fields::Named(fields) => {
      let mut ids = vec![];
      let mut names = vec![];
      let mut reader = vec![];
      let mut selfs = vec![];
      let len = fields.named.len() as u64;
      for (i, f) in fields.named.iter().enumerate() {
        let id = match find_id(&f.attrs) {
          Some((_, id)) => id,
          None => i as u64,
        };
        names.push(f.ident.as_ref().unwrap());
        ids.push(id);
        reader.push(match find_must_exist(&f.attrs) {
          Some(_) => quote!(must_read),
          None => quote!(read),
        });
        if has_self {
          selfs.push(quote!(&self.));
        } else {
          selfs.push(quote!());
        }
      }
      (
        quote!({
          #(
            #names: m.#reader(#ids)?,
          )*
        }),
        len,
        quote!({
          #(
            m.write(#selfs #names)?;
          )*
          Ok(())
        }),
      )
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
    impl #impl_generics bb_transfer::MessageRead for #name #ty_generics #where_clause {
      fn read(m: &mut bb_transfer::MessageReader) -> Result<Self, bb_transfer::ReadError> {
        Ok(match m.read_u32()? {
          #(
            #variant_read
          )*
          v => panic!("unknown enum id {}", v),
        })
      }
    }
    impl #impl_generics bb_transfer::MessageWrite for #name #ty_generics #where_clause {
      fn write(&self, m: &mut bb_transfer::MessageWriter) -> Result<(), bb_transfer::WriteError> {
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
        impl #impl_generics bb_transfer::MessageRead for #name #ty_generics #where_clause {
          fn read(m: &mut bb_transfer::MessageReader) -> Result<Self, bb_transfer::ReadError> {
            Ok(Self {
              #(
                #field: m.read()?,
              )*
            })
          }
        }
        impl #impl_generics bb_transfer::MessageWrite for #name #ty_generics #where_clause {
          fn write(&self, m: &mut bb_transfer::MessageWriter) -> Result<(), bb_transfer::WriteError> {
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
