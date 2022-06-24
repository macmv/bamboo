mod iter;
pub use iter::AppendIters;

/// A code generator. It is used to generate the source files in build.rs
pub struct CodeGen {
  current:      String,
  // Indent level (not amount of spaces)
  indent:       usize,
  // Indent is added when we write a new line, not on write_line
  needs_indent: bool,
  // Used when writing doc comments. Indents will be split between this index. For example, a
  // doc_comment of Some(2) and an indent of 3 would generate 4 spaces, then `/// `, then 2 spaces.
  doc_comment:  Option<usize>,
}
pub enum EnumVariant {
  Named(String),
  Tuple(String, Vec<String>),
  Struct(String, Vec<(String, String)>),
}
pub enum MatchBranch<'a> {
  /// A unit variant. Example:
  /// ```ignore
  /// match var {
  ///   Self::#name => /* ... */
  /// }
  /// ```
  Unit(&'a str),
  /// A tuple variant. Example:
  /// ```ignore
  /// match var {
  ///   Self::#name(#val1, #val2) => /* ... */
  /// }
  /// ```
  Tuple(&'a str, &'a [&'a str]),
  /// A struct variant. Example:
  /// ```ignore
  /// match var {
  ///   Self::#name { #field1, #field2 } => /* ... */
  /// }
  /// ```
  Struct(&'a str, Vec<String>),
  /// Anything else variant. Example:
  /// ```ignore
  /// match var {
  ///   _ => /* ... */
  /// }
  /// ```
  Other,
}
pub struct FuncArg<'a> {
  pub name: &'a str,
  pub ty:   &'a str,
}

impl CodeGen {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    CodeGen {
      current:      String::new(),
      indent:       0,
      needs_indent: false,
      doc_comment:  None,
    }
  }
  /// Sets the indent of this generator.
  pub fn set_indent(&mut self, indent: usize) { self.indent = indent; }
  /// Enables doc comments. All code written will be prefixed with `///`, at the
  /// current indent. So, future indents will be inserted after the current
  /// indent level. Disabling this will go back to writing normal code.
  pub fn set_doc_comment(&mut self, enable: bool) {
    if enable {
      self.doc_comment = Some(self.indent)
    } else {
      self.doc_comment = None
    }
  }
  /// Returns the current indent of this generator.
  pub fn indent(&self) -> usize { self.indent }
  /// Writes an enum literal. Example:
  /// ```
  /// # use bb_data::gen::{CodeGen, EnumVariant};
  /// # let mut gen = CodeGen::new();
  /// gen.write_enum("Hello", [
  ///   EnumVariant::Named("Nothing".into()),
  ///   EnumVariant::Tuple("Something".into(), vec!["String".into(), "i32".into()]),
  ///   EnumVariant::Struct("Complex".into(), vec![
  ///     ("name".into(), "String".into()),
  ///     ("amount".into(), "i32".into()),
  ///   ]),
  /// ].into_iter());
  /// # let out = gen.into_output();
  /// # eprintln!("OUTPUT: {}", out);
  /// # assert_eq!(out,
  /// # r#"pub enum Hello {
  /// #   Nothing,
  /// #   Something(String, i32),
  /// #   Complex {
  /// #     name: String,
  /// #     amount: i32,
  /// #   },
  /// # }
  /// # "#);
  /// ```
  /// That will produce:
  /// ```ignore
  /// pub enum Hello {
  ///   Nothing,
  ///   Something(String, i32),
  ///   Complex { name: String, amount: i32 },
  /// }
  /// ```
  pub fn write_enum(&mut self, name: &str, variants: impl Iterator<Item = EnumVariant>) {
    self.write("pub enum ");
    self.write(name);
    self.write_line(" {");
    self.add_indent();
    for variant in variants {
      variant.write(self);
    }
    self.remove_indent();
    self.write_line("}");
  }
  /// Writes an impl block. Example:
  /// ```
  /// # use bb_data::gen::{CodeGen, EnumVariant};
  /// # let mut gen = CodeGen::new();
  /// gen.write_impl("Hello", |gen| {
  ///   gen.write_line("pub fn hello_world() {}");
  /// });
  /// # let out = gen.into_output();
  /// # eprintln!("OUTPUT: {}", out);
  /// # assert_eq!(out,
  /// # r#"impl Hello {
  /// #   pub fn hello_world() {}
  /// # }
  /// # "#);
  /// ```
  /// That will produce:
  /// ```ignore
  /// impl Hello {
  ///   pub fn hello_world() {}
  /// }
  /// ```
  pub fn write_impl<F>(&mut self, name: &str, write_body: F)
  where
    F: FnOnce(&mut CodeGen),
  {
    self.write("impl ");
    self.write(name);
    self.write_line(" {");
    self.add_indent();
    write_body(self);
    self.remove_indent();
    self.write_line("}");
  }
  /// Writes a `mod` block. Example:
  /// ```
  /// # use bb_data::gen::{CodeGen, EnumVariant};
  /// # let mut gen = CodeGen::new();
  /// gen.write_mod("foo", |gen| {
  ///   gen.write_line("pub fn hello_world() {}");
  /// });
  /// # let out = gen.into_output();
  /// # eprintln!("OUTPUT: {}", out);
  /// # assert_eq!(out,
  /// # r#"pub mod foo {
  /// #   pub fn hello_world() {}
  /// # }
  /// # "#);
  /// ```
  /// That will produce:
  /// ```ignore
  /// pub mod foo {
  ///   pub fn hello_world() {}
  /// }
  /// ```
  pub fn write_mod<F>(&mut self, name: &str, write_body: F)
  where
    F: FnOnce(&mut CodeGen),
  {
    self.write("pub mod ");
    self.write(name);
    self.write_line(" {");
    self.add_indent();
    write_body(self);
    self.remove_indent();
    self.write_line("}");
  }
  /// Writes a function. Example:
  /// ```
  /// # use bb_data::gen::{CodeGen, FuncArg};
  /// # let mut gen = CodeGen::new();
  /// gen.write_func("my_func", &[
  ///   FuncArg { name: "name", ty: "String" },
  ///   FuncArg { name: "amount", ty: "i32" },
  /// ], None, |gen| {
  ///   gen.write_line("println!(\"hello world!\");");
  /// });
  ///
  /// gen.write_func("plus_two", &[
  ///   FuncArg { name: "value", ty: "i32" },
  /// ], Some("i32"), |gen| {
  ///   gen.write_line("value + 2");
  /// });
  /// # let out = gen.into_output();
  /// # eprintln!("OUTPUT: {}", out);
  /// # assert_eq!(out,
  /// # r#"pub fn my_func(name: String, amount: i32) {
  /// #   println!("hello world!");
  /// # }
  /// # pub fn plus_two(value: i32) -> i32 {
  /// #   value + 2
  /// # }
  /// # "#);
  /// ```
  /// That will produce:
  /// ```ignore
  /// pub fn my_func(name: String, amount: i32) {
  ///   println!("hello world!");
  /// }
  /// pub fn plus_two(value: i32) -> i32 {
  ///   value + 2
  /// }
  /// ```
  pub fn write_func<F>(&mut self, name: &str, args: &[FuncArg], ret: Option<&str>, write_body: F)
  where
    F: FnOnce(&mut CodeGen),
  {
    self.write("pub fn ");
    self.write(name);
    self.write("(");
    for (i, arg) in args.iter().enumerate() {
      arg.write(self);
      if i != args.len() - 1 {
        self.write(", ");
      }
    }
    self.write(")");
    if let Some(ret) = ret {
      self.write(" -> ");
      self.write(ret);
    }
    self.write_line(" {");
    self.add_indent();
    write_body(self);
    self.remove_indent();
    self.write_line("}");
  }
  /// Writes a match statement. Example:
  /// ```
  /// # use bb_data::gen::{CodeGen, FuncArg, MatchBranch};
  /// # let mut gen = CodeGen::new();
  /// gen.write_match("var", |gen| {
  ///   gen.write_match_branch(Some("Option"), MatchBranch::Unit("None"));
  ///   gen.write_line("println!(\"got nothing!\"),");
  ///   gen.write_match_branch(None, MatchBranch::Tuple("Some", &["value"]));
  ///   gen.write_line("println!(\"got something: {}!\", value),");
  /// });
  /// # let out = gen.into_output();
  /// # eprintln!("OUTPUT: {}", out);
  /// # assert_eq!(out,
  /// # r#"match var {
  /// #   Option::None => println!("got nothing!"),
  /// #   Some(value) => println!("got something: {}!", value),
  /// # }
  /// # "#);
  /// ```
  /// That will produce:
  /// ```ignore
  /// match var {
  ///   Option::None => println!("got index 0"),
  ///   Option::Some(value) => println!("got index 1"),
  /// }
  /// ```
  pub fn write_match<F>(&mut self, variable: &str, mut write_body: F)
  where
    F: FnMut(&mut CodeGen),
  {
    self.write("match ");
    self.write(variable);
    self.write_line(" {");
    self.add_indent();
    write_body(self);
    self.remove_indent();
    self.write_line("}");
  }
  /// See the docs for [`write_match`](Self::write_match).
  pub fn write_match_branch(&mut self, ty: Option<&str>, branch: MatchBranch) {
    if let Some(ty) = ty {
      self.write(ty);
      self.write("::");
    }
    branch.write(self);
  }
  /// Writes a block of code. Example:
  /// ```
  /// # use bb_data::gen::{CodeGen, FuncArg};
  /// # let mut gen = CodeGen::new();
  /// gen.write_block(|gen| {
  ///   gen.write_line("5 + 6");
  /// });
  /// # let out = gen.into_output();
  /// # eprintln!("OUTPUT: {}", out);
  /// # assert_eq!(out,
  /// # r#"{
  /// #   5 + 6
  /// # }
  /// # "#);
  /// ```
  /// That will produce:
  /// ```ignore
  /// {
  ///   5 + 6
  /// }
  /// ```
  pub fn write_block<F>(&mut self, write_block: F)
  where
    F: FnOnce(&mut CodeGen),
  {
    self.write_line("{");
    self.add_indent();
    write_block(self);
    self.remove_indent();
    self.write_line("}");
  }
  /// Writes a line comment. Example:
  /// ```
  /// # use bb_data::gen::CodeGen;
  /// # let mut gen = CodeGen::new();
  /// gen.write_comment("Hello world!");
  /// # let out = gen.into_output();
  /// # eprintln!("OUTPUT: {}", out);
  /// # assert_eq!(out,
  /// # r#"// Hello world!
  /// # "#);
  /// ```
  /// That will produce:
  #[allow(rustdoc::invalid_rust_codeblocks)]
  /// ```ignore
  /// // Hello world!
  /// ```
  pub fn write_comment(&mut self, text: &str) {
    self.write("// ");
    self.write_line(text);
  }

  pub fn write(&mut self, src: &str) {
    // Make sure not to indent when we aren't writing anything
    if src.is_empty() {
      return;
    }
    if self.needs_indent {
      if let Some(doc_comment) = self.doc_comment {
        self.current.push_str(&"  ".repeat(doc_comment));
        self.current.push_str("/// ");
        self.current.push_str(&"  ".repeat(self.indent - doc_comment));
      } else {
        self.current.push_str(&"  ".repeat(self.indent));
      }
      self.needs_indent = false;
    }
    self.current.push_str(src);
  }
  pub fn write_line(&mut self, src: &str) {
    // If we want a blank line, only add indents before doc_comment
    if src.is_empty() {
      if self.current.is_empty() {
        if let Some(doc_comment) = self.doc_comment {
          self.current.push_str(&"  ".repeat(doc_comment));
          // Note there is no trailing whitespace here
          self.current.push_str("///");
        }
      }
      self.current.push('\n');
      self.needs_indent = true;
    } else {
      self.write(src);
      self.current.push('\n');
      self.needs_indent = true;
    }
  }
  /// Adds a new indent level to the generator.
  pub fn add_indent(&mut self) { self.indent = self.indent.checked_add(1).unwrap(); }
  /// Removes a level of indent from the generator.
  pub fn remove_indent(&mut self) { self.indent = self.indent.checked_sub(1).unwrap(); }
  /// Clears all the indents from the generator.
  pub fn clear_indent(&mut self) { self.indent = 0; }
  /// Returns the code that was generated with this generator.
  pub fn into_output(self) -> String { self.current }
}

impl EnumVariant {
  pub fn write(&self, gen: &mut CodeGen) {
    match self {
      Self::Named(name) => {
        gen.write(name);
        gen.write_line(",");
      }
      Self::Tuple(name, fields) => {
        gen.write(name);
        gen.write("(");
        for (i, f) in fields.iter().enumerate() {
          gen.write(f);
          if i != fields.len() - 1 {
            gen.write(", ");
          }
        }
        gen.write_line("),");
      }
      Self::Struct(name, fields) => {
        gen.write(name);
        gen.write_line(" {");
        gen.add_indent();
        for (name, ty) in fields {
          gen.write(name);
          gen.write(": ");
          gen.write(ty);
          gen.write_line(",");
        }
        gen.remove_indent();
        gen.write_line("},");
      }
    }
  }
}
impl MatchBranch<'_> {
  pub fn write(&self, gen: &mut CodeGen) {
    match self {
      Self::Unit(name) => {
        gen.write(name);
      }
      Self::Tuple(name, fields) => {
        gen.write(name);
        gen.write("(");
        for (i, f) in fields.iter().enumerate() {
          gen.write(f);
          if i != fields.len() - 1 {
            gen.write(", ");
          }
        }
        gen.write(")");
      }
      Self::Struct(name, fields) => {
        gen.write(name);
        if fields.is_empty() {
          gen.write(" { .. }");
        } else {
          gen.write_line(" {");
          gen.add_indent();
          for name in fields {
            gen.write(name);
            gen.write_line(",");
          }
          gen.remove_indent();
          gen.write("}");
        }
      }
      Self::Other => {
        gen.write("_");
      }
    }
    gen.write(" => ");
  }
}
impl FuncArg<'_> {
  pub fn slf_ref() -> Self { FuncArg { name: "&self", ty: "" } }
  pub fn write(&self, gen: &mut CodeGen) {
    gen.write(self.name);
    if !self.ty.is_empty() {
      gen.write(": ");
      gen.write(self.ty);
    }
  }
}

use std::fmt;
impl fmt::Debug for CodeGen {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.debug_struct("CodeGen").finish() }
}
