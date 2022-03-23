use super::{simplify, Expr, Field, Instr, Op, Packet, RType, Type, Value, VarBlock};

fn call(name: &str, args: Vec<Expr>) -> Op { Op::Call("tcp::Packet".into(), name.into(), args) }
fn field(name: &str) -> Expr { Expr::new(Value::Field(name.into())) }
fn as_(name: &str) -> Op { Op::As(name.into()) }
fn packet() -> Expr { Expr::new(Value::packet_var()) }
fn block(block: Vec<Instr>) -> VarBlock { VarBlock { vars: vec![], block } }

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
    reader:  block(reader),
    writer:  block(vec![]),
  };
  simplify::finish(&mut p);
  p.find_reader_types_gen_writer();

  assert_eq!(p.fields, fields);
  assert_eq!(p.writer.block, writer);
}
