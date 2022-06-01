use proc_macro2::TokenStream as TokenStream2;
use syn::{
  AttrStyle, Attribute, Expr, Field, Fields, ItemEnum, ItemStruct, Path, Type, Variant, Visibility,
};

pub fn gen_docs<'a, T: 'a>(input: &'a T) -> String
where
  Source<'a, T>: fmt::Display,
{
  format!("```rust\n{}\n```", Source(input))
}

pub struct Source<'a, T>(&'a T);

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
impl fmt::Display for Source<'_, ItemEnum> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for attr in &self.0.attrs {
      writeln!(f, "{}", Source(attr))?;
    }
    writeln!(f, "{}enum {} {{", Source(&self.0.vis), self.0.ident)?;
    for variant in &self.0.variants {
      writeln!(f, "{}", Source(variant))?;
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
    write!(f, "  {}{:?}: {},", Source(&self.0.vis), self.0.ident.as_ref(), Source(&self.0.ty))?;
    Ok(())
  }
}
impl fmt::Display for Source<'_, Variant> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for attr in &self.0.attrs {
      writeln!(f, "  {}", Source(attr))?;
    }
    write!(f, "  {}", &self.0.ident)?;
    match &self.0.fields {
      Fields::Named(named) => {
        writeln!(f, "{{")?;
        for field in &named.named {
          writeln!(f, "  {}", Source(field))?;
        }
        writeln!(f, "  }},")?;
      }
      Fields::Unnamed(unnamed) => {
        write!(f, "(")?;
        for field in &unnamed.unnamed {
          writeln!(f, "{}", Source(field))?;
        }
        write!(f, "),")?;
      }
      Fields::Unit => writeln!(f, ",")?,
    }
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
