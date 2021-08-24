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
  /// ```
  /// match var {
  ///   Self::#name => /* ... */
  /// }
  /// ```
  Unit(String),
  /// A tuple variant. Example:
  /// ```
  /// match var {
  ///   Self::#name(#val1, #val2) => /* ... */
  /// }
  /// ```
  Tuple(String, Vec<String>),
  /// A struct variant. Example:
  /// ```
  /// match var {
  ///   Self::#name { #field1, #field2 } => /* ... */
  /// }
  /// ```
  Struct(String, Vec<String>),
  Other,
}

impl CodeGen {
  pub fn new() -> Self {
    CodeGen { current: String::new(), indent: 0, needs_indent: true }
  }
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
