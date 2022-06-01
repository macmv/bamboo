use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
  parse_macro_input, punctuated::Punctuated, token::Bracket, AttrStyle, Attribute, Expr, Field,
  Fields, GenericArgument, Ident, ItemStruct, Path, Token, Type, Visibility,
};

#[allow(clippy::collapsible_match)]
pub fn ctype(_args: TokenStream, input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as ItemStruct);

  let original_docs = gen_docs(&input);
  let mut changed = false;

  match &mut input.fields {
    Fields::Named(fields) => {
      let mut new_fields = Punctuated::<Field, Token![,]>::new();
      for mut field in fields.named.clone() {
        if let Some(host_type) = host_type(&field.ty) {
          let mut not_host = field.clone();
          field.ty = host_type;
          field.attrs.push(Attribute {
            pound_token:   Token![#](Span::call_site()),
            style:         AttrStyle::Outer,
            bracket_token: Bracket { span: Span::call_site() },
            path:          path(&["cfg"]),
            tokens:        quote!((feature = "host")),
          });
          not_host.attrs.push(Attribute {
            pound_token:   Token![#](Span::call_site()),
            style:         AttrStyle::Outer,
            bracket_token: Bracket { span: Span::call_site() },
            path:          path(&["cfg"]),
            tokens:        quote!((not(feature = "host"))),
          });
          new_fields.push(field);
          new_fields.push(not_host);
          changed = true;
        } else {
          new_fields.push(field);
        }
      }
      fields.named = new_fields;
    }
    _ => {}
  };

  let input_attrs = input.attrs.clone();
  input.attrs.clear();
  let ty = &input.ident;

  let docs = if changed {
    let generated_docs = gen_docs(&input);
    quote! {
      #[doc = "Original struct:"]
      #[doc = #original_docs]
      #[doc = "Generated struct:"]
      #[doc = #generated_docs]
    }
  } else {
    quote!()
  };

  quote! {
    #(#input_attrs)*
    #docs
    #[repr(C)]
    #[derive(Clone)]
    #input

    #[cfg(feature = "host")]
    impl Copy for #ty {}
    #[cfg(feature = "host")]
    unsafe impl wasmer::ValueType for #ty {}
  }
  .into()
}

fn gen_docs(input: &ItemStruct) -> String { format!("```rust\n{}\n```", Source(input)) }

struct Source<'a, T>(&'a T);

use std::fmt;

impl fmt::Display for Source<'_, ItemStruct> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for attr in &self.0.attrs {
      writeln!(f, "{}", Source(attr))?;
    }
    writeln!(f, "{}struct {} {{", Source(&self.0.vis), self.0.ident)?;
    for field in &self.0.fields {
      writeln!(f, "{}", Source(field))?;
    }
    writeln!(f, "}}")?;
    Ok(())
  }
}
impl fmt::Display for Source<'_, Visibility> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.0 {
      Visibility::Inherited => Ok(()),
      Visibility::Public(_) => write!(f, "pub "),
      Visibility::Crate(_) => write!(f, "pub(crate) "),
      _ => Ok(()),
    }
  }
}
impl fmt::Display for Source<'_, Attribute> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if matches!(self.0.path.get_ident(), Some(path) if path == "doc") {
      let doc = format!("{}", self.0.tokens);
      let doc = &doc[3..doc.len() - 1];
      let mut unescaped = String::with_capacity(doc.len());
      let mut skip = false;
      for c in doc.chars() {
        if !skip && c == '\\' {
          skip = true;
          continue;
        }
        skip = false;
        unescaped.push(c);
      }
      write!(f, "///{unescaped}")?;
    } else {
      write!(f, "#")?;
      match self.0.style {
        AttrStyle::Inner(_) => write!(f, "!")?,
        _ => {}
      }
      write!(f, "[")?;
      write!(f, "{}", Source(&self.0.path))?;
      write!(f, "{}", Source(&self.0.tokens))?;
      write!(f, "]")?;
    }
    Ok(())
  }
}
impl fmt::Display for Source<'_, Field> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for attr in &self.0.attrs {
      writeln!(f, "  {}", Source(attr))?;
    }
    write!(
      f,
      "  {}{}: {},",
      Source(&self.0.vis),
      self.0.ident.as_ref().unwrap(),
      Source(&self.0.ty)
    )?;
    Ok(())
  }
}
impl fmt::Display for Source<'_, Type> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.0 {
      Type::Array(ty) => write!(f, "[{}; {}]", Source(&*ty.elem), Source(&ty.len)),
      Type::Path(ty) => write!(f, "{}", Source(&ty.path)),
      Type::Ptr(ty) => write!(f, "*const {}", Source(&*ty.elem)),
      _ => Ok(()),
    }
  }
}
impl fmt::Display for Source<'_, Expr> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.0 {
      Expr::Lit(lit) => match &lit.lit {
        syn::Lit::Int(lit) => write!(f, "{}", lit.base10_digits()),
        _ => Ok(()),
      },
      _ => Ok(()),
    }
  }
}
impl fmt::Display for Source<'_, Path> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.0.leading_colon.is_some() {
      write!(f, "::")?;
    }
    for pair in self.0.segments.pairs() {
      write!(f, "{}", pair.value().ident)?;
      match &pair.value().arguments {
        syn::PathArguments::AngleBracketed(args) => {
          write!(f, "<")?;
          for pair in args.args.pairs() {
            match pair.value() {
              syn::GenericArgument::Type(ty) => write!(f, "{}", Source(ty))?,
              _ => {}
            }
            if pair.punct().is_some() {
              write!(f, ", ")?;
            }
          }
          write!(f, ">")?;
        }
        _ => {}
      }
      if pair.punct().is_some() {
        write!(f, "::")?;
      }
    }
    Ok(())
  }
}
impl fmt::Display for Source<'_, TokenStream2> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.0.fmt(f) }
}

fn path(text: &[&str]) -> Path {
  let mut segments = Punctuated::new();
  for segment in text {
    segments.push(syn::PathSegment {
      ident:     Ident::new(segment, Span::call_site()),
      arguments: syn::PathArguments::None,
    });
  }
  Path { leading_colon: None, segments }
}
fn host_type(ty: &Type) -> Option<Type> {
  match ty {
    Type::Ptr(ty) => Some(Type::Path(syn::TypePath {
      qself: None,
      path:  {
        let mut segments = Punctuated::new();
        segments.push(syn::PathSegment {
          ident:     Ident::new("wasmer", Span::call_site()),
          arguments: syn::PathArguments::None,
        });
        segments.push(syn::PathSegment {
          ident:     Ident::new("WasmPtr", Span::call_site()),
          arguments: syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token:     Token![<](Span::call_site()),
            args:         {
              let mut args = Punctuated::new();
              args.push(GenericArgument::Type(*ty.elem.clone()));
              args
            },
            gt_token:     Token![>](Span::call_site()),
          }),
        });
        Path { leading_colon: None, segments }
      },
    })),
    _ => None,
  }
}
