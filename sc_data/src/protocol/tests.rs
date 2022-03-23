use super::{
  simplify, Cond, Expr, Field, Instr, Lit, Op, Packet, RType, Type, Value, VarBlock, VarKind,
};

fn call(name: &str, args: Vec<Expr>) -> Op { Op::Call("tcp::Packet".into(), name.into(), args) }
fn cond(expr: Expr) -> Cond { Cond::Bool(expr) }
fn field(name: &str) -> Expr { Expr::new(Value::Field(name.into())) }
fn lit(lit: impl Into<Lit>) -> Expr { Expr::new(Value::Lit(lit.into())) }
fn local(id: usize) -> Expr { Expr::new(Value::Var(id)) }
fn as_(name: &str) -> Op { Op::As(name.into()) }
fn packet() -> Expr { Expr::new(Value::packet_var()) }
fn block(block: Vec<Instr>, locals: usize) -> VarBlock {
  let mut vars = vec![VarKind::This];
  for _ in 0..locals {
    vars.push(VarKind::Local);
  }
  VarBlock { vars, block }
}

macro_rules! call {
  ( $name:ident [ $($arg:expr),* ] ) => {
    call(stringify!($name), vec![$($arg),*])
  }
}

macro_rules! fields {
  [ $($name:ident: $ty:ident),* ] => {
    vec![ $( Field {
      name: stringify!($name).into(),
      ty: Type::$ty,
      reader_type: None,
      initialized: false,
      option: false,
    } ),* ]
  }
}

fn generate(p: &mut Packet) {
  simplify::finish(p);
  p.find_reader_types_gen_writer();
}

#[test]
fn simple_writer_test() {
  let reader = vec![
    Instr::Set("foo".into(), packet().op(call!(read_i8[]))),
    Instr::Set("bar".into(), packet().op(call!(read_i16[]))),
    Instr::Set("baz".into(), packet().op(call!(read_i32[]))),
  ];
  let writer = vec![
    Instr::Expr(packet().op(call!(write_i8[field("foo").op(Op::Deref).op(as_("i8"))]))),
    Instr::Expr(packet().op(call!(write_i16[field("bar").op(Op::Deref).op(as_("i16"))]))),
    Instr::Expr(packet().op(call!(write_i32[field("baz").op(Op::Deref)]))),
  ];
  let fields = vec![
    Field {
      name:        "foo".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i8")),
      initialized: true,
      option:      false,
    },
    Field {
      name:        "bar".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i16")),
      initialized: true,
      option:      false,
    },
    Field {
      name:        "baz".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i32")),
      initialized: true,
      option:      false,
    },
  ];
  let mut p = Packet {
    extends: "".into(),
    class:   "".into(),
    name:    "Bar".into(),
    fields:  fields![foo: Int, bar: Int, baz: Int],
    reader:  block(reader, 0),
    writer:  block(vec![], 0),
  };
  generate(&mut p);

  assert_eq!(p.fields, fields);
  assert_eq!(p.writer.block, writer);
}

#[test]
fn conditional_writer_test() {
  let reader = vec![
    Instr::Set("foo".into(), packet().op(call!(read_i32[]))),
    Instr::If(
      Cond::Greater(field("foo"), lit(0)),
      vec![Instr::Set("baz".into(), packet().op(call!(read_i32[])))],
      vec![],
    ),
  ];
  let writer = vec![
    Instr::Expr(packet().op(call!(write_i32[field("foo").op(Op::Deref)]))),
    Instr::If(
      Cond::Greater(field("foo"), lit(0)),
      vec![Instr::Expr(packet().op(call!(write_i32[field("baz").op(Op::Deref)])))],
      vec![],
    ),
  ];
  let fields = vec![
    Field {
      name:        "foo".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i32")),
      initialized: true,
      option:      false,
    },
    Field {
      name:        "baz".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i32")),
      initialized: false,
      option:      true,
    },
  ];
  let mut p = Packet {
    extends: "".into(),
    class:   "".into(),
    name:    "Bar".into(),
    fields:  fields![foo: Int, bar: Int, baz: Int],
    reader:  block(reader, 0),
    writer:  block(vec![], 0),
  };
  generate(&mut p);

  assert_eq!(p.fields, fields);
  assert_eq!(p.writer.block, writer);

  let reader = vec![
    Instr::Let(1, packet().op(call!(read_i32[]))),
    Instr::If(
      Cond::Greater(local(1), lit(0)),
      vec![Instr::Set("baz".into(), packet().op(call!(read_i32[])))],
      vec![],
    ),
  ];
  let writer = vec![
    Instr::Expr(packet().op(call!(
      write_i32[lit(0).op(Op::If(Box::new(cond(field("baz").op(call!(is_some[])))), lit(1)))]
    ))),
    Instr::If(
      cond(field("baz").op(call!(is_some[]))),
      vec![Instr::Expr(packet().op(call!(write_i32[field("baz").op(Op::Deref)])))],
      vec![],
    ),
  ];
  let fields = vec![Field {
    name:        "baz".into(),
    ty:          Type::Int,
    reader_type: Some(RType::new("i32")),
    initialized: false,
    option:      true,
  }];
  let mut p = Packet {
    extends: "".into(),
    class:   "".into(),
    name:    "Bar".into(),
    fields:  fields![foo: Int, bar: Int, baz: Int],
    reader:  block(reader, 1),
    writer:  block(vec![], 0),
  };
  generate(&mut p);

  assert_eq!(p.writer.block, writer);
  assert_eq!(p.fields, fields);
}
