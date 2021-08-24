/// A code generator. It is used to generate the source files in build.rs
pub struct CodeGen {
  current:      String,
  // Indent level (not amount of spaces)
  indent:       usize,
  // Indent is added when we write a new line, not on write_line
  needs_indent: bool,
}
pub enum EnumVariant {
  Named(String),
  Tuple(String, Vec<String>),
  Struct(String, Vec<(String, String)>),
}
pub enum MatchBranch {
  /// A unit variant. Example:
  /// ```ignore
  /// match var {
  ///   Self::#name => /* ... */
  /// }
  /// ```
  Unit(String),
  /// A tuple variant. Example:
  /// ```ignore
  /// match var {
  ///   Self::#name(#val1, #val2) => /* ... */
  /// }
  /// ```
  Tuple(String, Vec<String>),
  /// A struct variant. Example:
  /// ```ignore
  /// match var {
  ///   Self::#name { #field1, #field2 } => /* ... */
  /// }
  /// ```
  Struct(String, Vec<String>),
  /// Anything else variant. Example:
  /// ```ignore
  /// match var {
  ///   _ => /* ... */
  /// }
  /// ```
  Other,
}
pub struct FuncArg<'a> {
  name: &'a str,
  ty:   &'a str,
}

impl CodeGen {
  pub fn new() -> Self {
    CodeGen { current: String::new(), indent: 0, needs_indent: true }
  }
  /// Writes an enum literal. Example:
  /// ```
  /// # use data::gen::{CodeGen, EnumVariant};
  /// # let mut gen = CodeGen::new();
  /// gen.write_enum("Hello", &[
  ///   EnumVariant::Named("Nothing"),
  ///   EnumVariant::Tuple("Something", vec!["String", "i32"]),
  ///   EnumVariant::Struct("Complex", vec![("name", "String"), ("amount", "i32")]),
  /// ])
  /// ```
  /// That will produce:
  /// ```ignore
  /// pub enum Hello {
  ///   Nothing,
  ///   Something(String, i32),
  ///   Complex { name: String, amount: i32 },
  /// }
  /// ```
  pub fn write_enum(&mut self, name: &str, variants: &[EnumVariant]) {
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
  /// Writes a function. Example:
  /// ```
  /// # use data::gen::{CodeGen, FuncArg};
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
    for arg in args {
      arg.write(self);
    }
    self.write(") ");
    if let Some(ret) = ret {
      self.write("-> ");
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
  /// # use data::gen::{CodeGen, MatchBranch};
  /// # let mut gen = CodeGen::new();
  /// gen.write_match("var", "Option", &[
  ///   MatchBranch::Unit("None"),
  ///   MatchBranch::Tuple("Some", &["value"]),
  /// ], |gen, i| {
  ///   gen.write("println!(\"got index ");
  ///   gen.write(i.to_string());
  ///   gen.write("\"),");
  /// });
  ///
  /// gen.write_func("plus_two", &[
  ///   FuncArg { name: "value", ty: "i32" },
  /// ], Some("i32"), |gen| {
  ///   gen.write_line("value + 2");
  /// });
  /// # assert_eq!(gen.into_output(),
  /// # r#"match var {
  /// #   Option::None => println!("got index 0"),
  /// #   Option::Some(value) => println!("got index 1"),
  /// # }"#;
  /// ```
  /// That will produce:
  /// ```ignore
  /// match var {
  ///   Option::None => println!("got index 0"),
  ///   Option::Some(value) => println!("got index 1"),
  /// }
  /// ```
  pub fn write_match<F>(
    &mut self,
    variable: &str,
    type_name: &str,
    branches: &[MatchBranch],
    mut write_block: F,
  ) where
    F: FnMut(&mut CodeGen, usize),
  {
    self.write("match ");
    self.write(variable);
    self.write_line(" {");
    self.add_indent();
    for (i, branch) in branches.iter().enumerate() {
      if let MatchBranch::Other = branch {
        branch.write(self);
      } else {
        branch.write(self);
        self.write(type_name);
        self.write_line("::");
      }
      write_block(self, i);
    }
    self.remove_indent();
    self.write_line("}");
  }

  pub fn write(&mut self, src: &str) {
    if self.needs_indent {
      self.current.push_str(&"  ".repeat(self.indent));
      self.needs_indent = false;
    }
    self.current.push_str(src);
  }
  pub fn write_line(&mut self, src: &str) {
    self.current.push_str(src);
    self.current.push_str("\n");
  }
  /// Adds a new indent level to the generator.
  pub fn add_indent(&mut self) {
    self.indent = self.indent.checked_add(1).unwrap();
  }
  /// Removes a level of indent from the generator.
  pub fn remove_indent(&mut self) {
    self.indent = self.indent.checked_sub(1).unwrap();
  }
  /// Clears all the indents from the generator.
  pub fn clear_indent(&mut self) {
    self.indent = 0;
  }
}

impl EnumVariant {
  pub fn write(&self, gen: &mut CodeGen) {
    match self {
      Self::Named(name) => {
        gen.write(&name);
        gen.write_line(",");
      }
      Self::Tuple(name, fields) => {
        gen.write(&name);
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
        gen.write(&name);
        gen.write("(");
        gen.add_indent();
        for (name, ty) in fields {
          gen.write(name);
          gen.write(": ");
          gen.write(ty);
          gen.write_line(",");
        }
        gen.remove_indent();
        gen.write_line("),");
      }
    }
  }
}
impl MatchBranch {
  pub fn write(&self, gen: &mut CodeGen) {
    match self {
      Self::Unit(name) => {
        gen.write(&name);
      }
      Self::Tuple(name, fields) => {
        gen.write(&name);
        gen.write("(");
        for (i, f) in fields.iter().enumerate() {
          gen.write(f);
          if i != fields.len() - 1 {
            gen.write(", ");
          }
        }
        gen.write_line(")");
      }
      Self::Struct(name, fields) => {
        gen.write(&name);
        gen.write(" { ");
        gen.add_indent();
        for name in fields {
          gen.write(name);
          gen.write_line(",");
        }
        gen.remove_indent();
        gen.write_line(" }");
      }
      Self::Other => {
        gen.write("_ =>");
      }
    }
    gen.write_line(" => ");
  }
}
impl FuncArg<'_> {
  pub fn write(&self, gen: &mut CodeGen) {
    gen.write(self.name);
    gen.write(": ");
    gen.write(self.ty);
  }
}