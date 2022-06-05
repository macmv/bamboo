use proc_macro2::TokenStream as TokenStream2;
use syn::{
  AttrStyle, Attribute, Expr, Field, Fields, ItemEnum, ItemStruct, ItemUnion, Path, Type, Variant,
  Visibility,
};

pub fn gen_docs<'a, T: 'a>(input: &'a T) -> String
where
  Source<'a, T>: fmt::Display,
{
  format!("```rust\n{}\n```", Source(input))
}

pub struct Writer<'a, 'b> {
  f:            &'a mut fmt::Formatter<'b>,
  indent:       u32,
  needs_indent: bool,
}

impl Writer<'_, '_> {
  pub fn indent(&mut self) { self.indent += 1; }
  pub fn unindent(&mut self) { self.indent -= 1; }
  pub fn write_src<T: SourceFmt>(&mut self, src: &T) -> fmt::Result { src.fmt(self) }
}

pub struct Source<'a, T>(&'a T);

pub trait SourceFmt {
  fn fmt(&self, f: &mut Writer) -> fmt::Result;
}

impl<T: SourceFmt> fmt::Display for Source<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut w = Writer { f, indent: 0, needs_indent: false };
    self.0.fmt(&mut w)
  }
}

use std::fmt;

impl Writer<'_, '_> {
  fn check_indent(&mut self) -> fmt::Result {
    if self.needs_indent {
      self.needs_indent = false;
      for _ in 0..self.indent {
        self.f.write_str("  ")?;
      }
    }
    Ok(())
  }
}

use fmt::Write;
impl Write for Writer<'_, '_> {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    let mut iter = s.split('\n').peekable();
    while let Some(line) = iter.next() {
      if !line.is_empty() {
        self.check_indent()?;
      }
      self.f.write_str(line)?;
      if iter.peek().is_some() {
        self.f.write_char('\n')?;
        self.needs_indent = true;
      }
    }
    Ok(())
  }
  fn write_char(&mut self, c: char) -> fmt::Result {
    if c != '\n' {
      self.check_indent()?;
    }
    self.f.write_char(c)?;
    self.needs_indent = c == '\n';
    Ok(())
  }
  fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
    self.write_str(&args.to_string())
  }
}

impl SourceFmt for ItemStruct {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    for attr in &self.attrs {
      f.write_src(attr)?;
      writeln!(f)?;
    }
    f.write_src(&self.vis)?;
    writeln!(f, "struct {} {{", self.ident)?;
    f.indent();
    for field in &self.fields {
      f.write_src(field)?;
      writeln!(f, ",")?;
    }
    f.unindent();
    writeln!(f, "}}")?;
    Ok(())
  }
}
impl SourceFmt for ItemUnion {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    for attr in &self.attrs {
      f.write_src(attr)?;
      writeln!(f)?;
    }
    f.write_src(&self.vis)?;
    writeln!(f, "union {} {{", self.ident)?;
    f.indent();
    for field in &self.fields.named {
      f.write_src(field)?;
      writeln!(f, ",")?;
    }
    f.unindent();
    writeln!(f, "}}")?;
    Ok(())
  }
}
impl SourceFmt for ItemEnum {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    for attr in &self.attrs {
      f.write_src(attr)?;
      writeln!(f)?;
    }
    f.write_src(&self.vis)?;
    writeln!(f, "enum {} {{", self.ident)?;
    f.indent();
    for variant in &self.variants {
      f.write_src(variant)?;
      writeln!(f, ",")?;
    }
    f.unindent();
    writeln!(f, "}}")?;
    Ok(())
  }
}
impl SourceFmt for Visibility {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    match self {
      Visibility::Inherited => Ok(()),
      Visibility::Public(_) => write!(f, "pub "),
      Visibility::Crate(_) => write!(f, "pub(crate) "),
      _ => Ok(()),
    }
  }
}
impl SourceFmt for Attribute {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    if matches!(self.path.get_ident(), Some(path) if path == "doc") {
      let doc = format!("{}", self.tokens);
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
      match self.style {
        AttrStyle::Inner(_) => write!(f, "!")?,
        _ => {}
      }
      write!(f, "[")?;
      f.write_src(&self.path)?;
      let mut iter = self.tokens.clone().into_iter();
      // If we have the #[foo = "bar"] syntax, we want a space before the token
      // stream.
      if let Some(first) = iter.next() {
        if syn::parse2::<syn::Token![=]>(first.into()).is_ok() {
          write!(f, " ")?;
        }
      }
      f.write_src(&self.tokens)?;
      write!(f, "]")?;
    }
    Ok(())
  }
}
impl SourceFmt for Field {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    for attr in &self.attrs {
      f.write_src(attr)?;
      writeln!(f)?;
    }
    if let Some(i) = self.ident.as_ref() {
      f.write_src(&self.vis)?;
      write!(f, "{i}: ")?;
    }
    f.write_src(&self.ty)?;
    Ok(())
  }
}
impl SourceFmt for Variant {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    for attr in &self.attrs {
      f.write_src(attr)?;
      writeln!(f)?;
    }
    write!(f, "{}", &self.ident)?;
    match &self.fields {
      Fields::Named(named) => {
        writeln!(f, " {{")?;
        f.indent();
        for pair in named.named.pairs() {
          f.write_src(*pair.value())?;
          writeln!(f, ",")?;
        }
        f.unindent();
        write!(f, "}}")?;
      }
      Fields::Unnamed(unnamed) => {
        write!(f, "(")?;
        for pair in unnamed.unnamed.pairs() {
          f.write_src(*pair.value())?;
          if pair.punct().is_some() {
            write!(f, ", ")?;
          }
        }
        write!(f, ")")?;
      }
      Fields::Unit => (),
    }
    Ok(())
  }
}
impl SourceFmt for Type {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    match self {
      Type::Array(ty) => write!(f, "[{}; {}]", Source(&*ty.elem), Source(&ty.len)),
      Type::Path(ty) => write!(f, "{}", Source(&ty.path)),
      Type::Ptr(ty) => write!(f, "*const {}", Source(&*ty.elem)),
      Type::Tuple(ty) => {
        write!(f, "(")?;
        for pair in ty.elems.pairs() {
          f.write_src(*pair.value())?;
          if pair.punct().is_some() {
            write!(f, ", ")?;
          }
        }
        write!(f, ")")
      }
      _ => Ok(()),
    }
  }
}
impl SourceFmt for Expr {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    match self {
      Expr::Lit(lit) => match &lit.lit {
        syn::Lit::Int(lit) => write!(f, "{}", lit.base10_digits()),
        _ => Ok(()),
      },
      _ => Ok(()),
    }
  }
}
impl SourceFmt for Path {
  fn fmt(&self, f: &mut Writer) -> fmt::Result {
    if self.leading_colon.is_some() {
      write!(f, "::")?;
    }
    for pair in self.segments.pairs() {
      write!(f, "{}", pair.value().ident)?;
      match &pair.value().arguments {
        syn::PathArguments::AngleBracketed(args) => {
          write!(f, "<")?;
          for pair in args.args.pairs() {
            match pair.value() {
              syn::GenericArgument::Type(ty) => f.write_src(ty)?,
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
impl SourceFmt for TokenStream2 {
  fn fmt(&self, f: &mut Writer) -> fmt::Result { <Self as fmt::Display>::fmt(self, f.f) }
}
