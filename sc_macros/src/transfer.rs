use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

use syn::{
  braced, parenthesized,
  parse::{Parse, ParseStream, Result},
  parse_macro_input,
  punctuated::Punctuated,
  token, Generics, Ident, LitInt, Token, Type, Visibility,
};

#[derive(Debug)]
enum TransferInput {
  Struct(Struct),
  Enum(Enum),
}

#[derive(Debug)]
struct Struct {
  vis:      Visibility,
  strct:    Token![struct],
  name:     Ident,
  generics: Generics,
  brace:    token::Brace,
  fields:   Punctuated<Field, Token![,]>,
}
#[derive(Debug)]
struct Enum {
  vis:      Visibility,
  strct:    Token![enum],
  name:     Ident,
  generics: Generics,
  brace:    token::Brace,
  variants: Punctuated<Variant, Token![,]>,
}
#[derive(Debug)]
struct Variant {
  name: Ident,
  data: VariantData,
}
#[derive(Debug)]
enum VariantData {
  None,
  Tuple { paren: token::Paren, fields: Punctuated<Type, Token![,]> },
  Struct { brace: token::Brace, fields: Punctuated<Field, Token![,]> },
}
#[derive(Debug)]
struct Field {
  // Here is where we parse something different:
  // ```
  // struct Foo {
  //   0 -> a: i32,
  //   1 -> b: String,
  //   2 -> c: (u8, u8),
  // }
  // ```
  //
  // The number is the ID of each field. This allows for forwards compatibility when we update the
  // proxy (but not the servers in the back).
  number: LitInt,
  arrow:  Token![->],
  name:   Ident,
  colon:  Token![:],
  ty:     Type,
}

impl Parse for TransferInput {
  fn parse(input: ParseStream) -> Result<Self> {
    /*
    let mut attrs = input.call(Attribute::parse_outer)?;
    let ahead = input.fork();
    let vis: Visibility = input.parse()?;
    */
    if input.peek(Token![struct]) {
      input.parse().map(TransferInput::Struct)
    } else {
      input.parse().map(TransferInput::Enum)
    }
  }
}

impl Parse for Struct {
  fn parse(input: ParseStream) -> Result<Self> {
    let content;
    Ok(Struct {
      vis:      input.parse()?,
      strct:    input.parse()?,
      name:     input.parse()?,
      generics: input.parse()?,
      brace:    braced!(content in input),
      fields:   content.parse_terminated(Field::parse)?,
    })
  }
}
impl Parse for Enum {
  fn parse(input: ParseStream) -> Result<Self> {
    let content;
    Ok(Enum {
      vis:      input.parse()?,
      strct:    input.parse()?,
      name:     input.parse()?,
      generics: input.parse()?,
      brace:    braced!(content in input),
      variants: content.parse_terminated(Variant::parse)?,
    })
  }
}
impl Parse for Variant {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(Variant { name: input.parse()?, data: input.parse()? })
  }
}
impl Parse for VariantData {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(if input.peek(token::Paren) {
      let content;
      VariantData::Tuple {
        paren:  parenthesized!(content in input),
        fields: content.parse_terminated(Type::parse)?,
      }
    } else if input.peek(token::Brace) {
      let content;
      VariantData::Struct {
        brace:  braced!(content in input),
        fields: content.parse_terminated(Field::parse)?,
      }
    } else {
      VariantData::None
    })
  }
}
impl Parse for Field {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(Field {
      number: input.parse()?,
      arrow:  input.parse()?,
      name:   input.parse()?,
      colon:  input.parse()?,
      ty:     input.parse()?,
    })
  }
}

impl ToTokens for TransferInput {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    match self {
      Self::Struct(s) => s.to_tokens(tokens),
      Self::Enum(e) => e.to_tokens(tokens),
    }
  }
}
impl ToTokens for Struct {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    self.vis.to_tokens(tokens);
    self.strct.to_tokens(tokens);
    self.name.to_tokens(tokens);
    self.generics.to_tokens(tokens);
    self.brace.surround(tokens, |tokens| self.fields.to_tokens(tokens));
  }
}
impl ToTokens for Enum {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    self.vis.to_tokens(tokens);
    self.strct.to_tokens(tokens);
    self.name.to_tokens(tokens);
    self.generics.to_tokens(tokens);
    self.brace.surround(tokens, |tokens| self.variants.to_tokens(tokens));
  }
}
impl ToTokens for Variant {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    self.name.to_tokens(tokens);
    self.data.to_tokens(tokens);
  }
}
impl ToTokens for VariantData {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    match self {
      Self::None => {}
      Self::Tuple { paren, fields } => {
        paren.surround(tokens, |tokens| fields.to_tokens(tokens));
      }
      Self::Struct { brace, fields } => {
        brace.surround(tokens, |tokens| fields.to_tokens(tokens));
      }
    }
  }
}
impl ToTokens for Field {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    self.name.to_tokens(tokens);
    self.colon.to_tokens(tokens);
    self.ty.to_tokens(tokens);
  }
}

pub fn transfer(input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(input as TransferInput);

  quote!(#args).into()
  /*
  match args {
    TransferInput::Enum(en) => t_enum(args.ident, args.generics, en.variants),
    TransferInput::Struct(s) => t_struct(args.ident, args.generics, s.fields),
  }
  */
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
