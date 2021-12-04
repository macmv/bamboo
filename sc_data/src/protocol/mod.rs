use crate::dl;
use serde::Deserialize;
use std::{fmt, io, path::Path};

pub mod convert;
mod gen;
mod simplify;
mod type_analysis;
mod writer;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  let mut versions = vec![];
  for &ver in crate::VERSIONS {
    let def: PacketDef = dl::get("protocol", ver);
    versions.push((ver, def));
  }
  gen::generate(versions, &out_dir.join("protocol"))?;
  Ok(())
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PacketDef {
  clientbound: Vec<Packet>,
  serverbound: Vec<Packet>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Type {
  /// Only present for return types
  Void,

  Byte,
  Char,
  Double,
  Float,
  Int,
  Long,
  Short,
  Bool,
  Class(String),
  Array(Box<Type>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Packet {
  /// The class this packet extends from.
  #[serde(default = "object_str")]
  pub extends: String,
  /// The class name of this packet.
  pub name:    String,
  /// A list of the fields in this packet.
  pub fields:  Vec<Field>,
  /// A list of instructs to read this packet. These are parsed from java
  /// bytecode, and translated into a more rust-like representation.
  pub reader:  VarBlock,
  /// The same format as the reader, but these instructions should be used for
  /// writing. There are a few differing instructions (like read/writer field),
  /// but the same `Instr` type should be used for both the reader and writer.
  pub writer:  VarBlock,

  /// The index in the data array that comes from sugarcane-data.
  #[serde(skip_deserializing)]
  pub tcp_id: i32,
}

impl PartialEq for Packet {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name && self.reader == other.reader
  }
}

fn object_str() -> String {
  "java/lang/Object".into()
}

/// The body of a function or closure. This includes a table of all variables to
/// their kind. This is what maps the variable ids to either `this`, an
/// argument, or a local variable.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct VarBlock {
  vars:  Vec<VarKind>,
  block: Vec<Instr>,
}

/// The kind of variable this is.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum VarKind {
  This,
  Arg,
  Local,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Field {
  /// The name of this field.
  pub name: String,
  /// The java type of this field.
  pub ty:   Type,

  /// The type based on the `reader` function.
  #[serde(skip_deserializing)]
  pub reader_type: Option<RType>,
  /// Set to true if this field is only set in certain conditionals.
  #[serde(skip_deserializing)]
  pub option:      bool,
  /// Set to true if this field is always initialized in all branches.
  #[serde(skip_deserializing)]
  pub initialized: bool,
}

/// A value. Can be a variable reference, a literal, or a function call.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Value {
  /// A null value. This should probably be converted to a `None` value in rust,
  /// but given how complex some of these readers are, it will be a pain to work
  /// with.
  Null,
  /// A literal value.
  Lit(Lit),
  /// A local variable.
  Var(usize),
  /// A packet field. Similar to a local variable, but may require `self.` or
  /// `this.` depending on the language.
  Field(String),
  /// A static field `1` on classs `0`.
  Static(String, String),
  /// An array, with a pre-determined length.
  Array(Box<Expr>),
  /// A static function call. The first item is the class, the second is the
  /// function, and the third is the arguments.
  CallStatic(String, String, Vec<Expr>),
  /// A refernce to a static method.
  MethodRef(String, String),
  /// A closure call. The first list is a list of arguments for the closure, and
  /// the second list is the instructions inside the closure.
  Closure(Vec<Expr>, VarBlock),
  /// This is what happens when we create a class in java. For all intensive
  /// purposes, it is a collection of data, that contains the given constructor
  /// arguments. The arguments must be executed in order.
  ///
  /// The name is the class name of the item being constructed. The mappings are
  /// usually descriptive enough, so this doesn't include any package
  /// information.
  New(String, Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Lit {
  Int(i32),
  Float(f32),
  String(String),
}

/// A rust-like instruction. This can map one-to-one with a subset of Rust
/// statements.
#[derive(Debug, Clone, Deserialize)]
pub enum Instr {
  /// This is a very simple call. If this is in the list of instructions, the
  /// entire reader from the superclass of this packet should be inserted here.
  Super,

  /// Sets a field to the given expression. This is the simples instruction, and
  /// it is by far the most common. In simple packets, the entire reader may
  /// just be a list of Set calls.
  Set(String, Expr),
  /// Sets a value in an array. The first item is the array, the second is the
  /// index, and the last one is the value to set it to.
  SetArr(Expr, Value, Expr),

  /// Declares a new variable, and assigns it to the given value. The index is a
  /// java internal feature, and it represents a unique id for each local
  /// variable. An implementation of this might simply call all variables
  /// `var0`, `var1`, etc.
  Let(usize, Expr),

  /// If the given conditional is true, then execute the first list of
  /// instructions. Otherwise, execute the second list.
  If(Cond, Vec<Instr>, Vec<Instr>),
  /// Iterates over the given range of numbers. The variable is a local
  /// variable, which is the value that should be used when iterating (for
  /// example, if var was Var(3), then this might be converted into `for var3 in
  /// ...`).
  For(usize, Range, Vec<Instr>),
  /// A switch statement. The list is a list of keys to blocks that should be
  /// executed. We require that every java switch block has a `break` call at
  /// the end of it.
  Switch(Expr, Vec<(i32, Vec<Instr>)>),

  /// Make sure the given string (the first item) is less than the given length
  /// (the second item). Any time you read a string, there is a max length.
  /// So, when writing, we should also verify the length. Making this a
  /// seperate instruction makes it easy to, for example, remove all the
  /// length checks in release mode.
  CheckStrLen(Expr, Value),

  /// Invokes the given expresison, and ignores the result. Used when we do
  /// things like call a function that returns void.
  Expr(Expr),

  /// Returns the given value.
  Return(Expr),
}

impl PartialEq for Instr {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Super, Self::Super) => true,
      // Field names change a bunch. If the reader ops are the same, it doesn't matter which field
      // they are going to.
      (Self::Set(_, a), Self::Set(_, b)) => a == b,
      (Self::SetArr(a, a1, a2), Self::SetArr(b, b1, b2)) => a == b && a1 == b1 && a2 == b2,
      (Self::Let(_, a), Self::Let(_, b)) => a == b,
      (Self::If(a, a1, a2), Self::If(b, b1, b2)) => a == b && a1 == b1 && a2 == b2,
      (Self::For(_, a, a1), Self::For(_, b, b1)) => a == b && a1 == b1,
      (Self::Switch(a, a1), Self::Switch(b, b1)) => a == b && a1 == b1,
      (Self::CheckStrLen(a, a1), Self::CheckStrLen(b, b1)) => a == b && a1 == b1,
      (Self::Expr(a), Self::Expr(b)) => a == b,
      (Self::Return(a), Self::Return(b)) => a == b,
      _ => false,
    }
  }
}

/// A range, used in a for loop.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Range {
  /// Start of the range, inclusive.
  min: Expr,
  /// End of range, exclusive.
  max: Expr,
}

/// An expression. Each operation should be applied in order, after the initial
/// value is found.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Expr {
  /// The initial value of this expression. This won't change, but at runtime is
  /// the initial value that will be used when processing the given operators.
  initial: Value,
  /// The operators applied to this expresion. Each operator should be applied
  /// in order, and will mutate the value of this expression.
  #[serde(default = "Vec::new")]
  ops:     Vec<Op>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Cond {
  Eq(Expr, Expr),
  Neq(Expr, Expr),
  Less(Expr, Expr),
  Greater(Expr, Expr),
  Lte(Expr, Expr),
  Gte(Expr, Expr),

  Or(Box<Cond>, Box<Cond>),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Op {
  /// Bitwise and with the given value.
  BitAnd(Expr),
  /// Shift right by the given value.
  Shr(Expr),
  /// Unsigned shift right by the given value.
  UShr(Expr),
  /// Shift left by the given value.
  Shl(Expr),

  /// Add the given value to the current value.
  Add(Expr),
  /// Divide the current value by the given amount.
  Div(Expr),

  /// Get the length of the current value. Only valid if the current value is an
  /// array.
  Len,
  /// Get the value at the given index in this array.
  Idx(Expr),
  /// Get the field on the value.
  Field(String),

  /// If the conditional is true, replace the current value with the given
  /// value. Otherwise, do not change the current value, or execute the given
  /// expr.
  If(Cond, Expr),

  /// Calls the given function (`1`) on the value. The class this function is on
  /// is element `0`.
  Call(String, String, Vec<Expr>),

  /// Casts to the given type.
  Cast(Type),

  /// Used for int to bool conversions. Not present in json.
  Neq(Expr),

  /// Used when casting in rust. Not present in json.
  As(RType),
}

/// A rust type.
#[derive(Clone, PartialEq, Deserialize)]
pub struct RType {
  name:     String,
  generics: Vec<RType>,
}

impl RType {
  pub fn new(name: impl Into<String>) -> Self {
    RType { name: name.into(), generics: vec![] }
  }
  pub fn generic(mut self, arg: impl Into<RType>) -> Self {
    self.generics.push(arg.into());
    self
  }
}

impl fmt::Debug for RType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut d = f.debug_tuple("RType");
    d.field(&self.name);
    if !self.generics.is_empty() {
      d.field(&self.generics);
    }
    d.finish()
  }
}
impl fmt::Display for RType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.name)?;
    if !self.generics.is_empty() {
      write!(f, "<")?;
    }
    for (i, g) in self.generics.iter().enumerate() {
      write!(f, "{}", g)?;
      if i != self.generics.len() - 1 {
        write!(f, ", ")?;
      }
    }
    if !self.generics.is_empty() {
      write!(f, ">")?;
    }
    Ok(())
  }
}

impl From<&str> for RType {
  fn from(s: &str) -> RType {
    RType::new(s)
  }
}

impl Type {
  pub fn to_rust(&self) -> RType {
    RType::new(match self {
      Self::Void => unreachable!(),
      Self::Bool => "bool",
      Self::Byte => "u8",
      Self::Short => "i16",
      Self::Int => "i32",
      Self::Long => "i64",
      Self::Float => "f32",
      Self::Double => "f64",
      Self::Char => "char",
      Self::Class(name) => return convert::class(name),
      Self::Array(ty) => return RType::new("Vec").generic(ty.to_rust()),
    })
  }
}

impl Op {
  pub fn precedence(&self) -> i32 {
    match self {
      Op::BitAnd(_) => 5,
      Op::Shr(_) => 4,
      Op::UShr(_) => 4,
      Op::Shl(_) => 4,

      Op::Div(_) => 3,
      Op::Add(_) => 2,

      Op::Cast(..) => 1,
      Op::As(..) => 1,

      Op::Neq(..) => 0,

      Op::Len => 0,
      Op::Idx(_) => 0,
      Op::Field(_) => 0,

      Op::If(..) => 0,
      Op::Call(..) => 0,
    }
  }
}

impl Expr {
  pub fn new(initial: Value) -> Expr {
    Expr { initial, ops: vec![] }
  }
  pub fn op(mut self, op: Op) -> Self {
    self.ops.push(op);
    self
  }
}

impl From<i32> for Lit {
  fn from(v: i32) -> Self {
    Lit::Int(v)
  }
}
