use super::{convert, Cond, Expr, Field, Instr, Op, Packet, Type, Value, VarBlock};

impl Packet {
  pub fn generate_writer(&mut self) {
    let mut gen = WriterGen::new(&self.reader, self.name.clone());
    gen.instr(&self.reader.block, &mut self.writer.block);
  }
}

struct WriterGen {
  vars:   Vec<Expr>,
  packet: String,
}

impl WriterGen {
  fn new(v: &VarBlock, packet: String) -> Self {
    WriterGen { vars: v.vars.iter().map(|_| Expr::new(Value::Null)).collect(), packet }
  }

  fn instr(&mut self, read: &[Instr], writer: &mut Vec<Instr>) {
    for i in read {
      match i {
        Instr::Set(field, expr) => {
          writer.push(self.set_expr(expr, field));
        }
        Instr::Let(i, expr) => self.vars[*i] = expr.clone(),
        Instr::Return(_) => {}
        _ => panic!("cannot convert {:?} into writer (packet {})", i, self.packet),
      }
    }
  }

  fn set_expr(&mut self, expr: &Expr, field: &str) -> Instr {
    assert_eq!(expr.initial, Value::Var(1), "unknown Set value: {:?}", expr);
    match expr.ops.first().unwrap() {
      Op::Call(class, name, _args) if class == "tcp::Packet" => {
        let writer_name = name.replace("read", "write");
        Instr::Expr(Expr::new(Value::Var(1)).op(Op::Call(
          class.clone(),
          writer_name,
          vec![Expr::new(Value::Field(field.into()))],
        )))
      }
      _ => panic!("cannot convert {:?} into writer (packet {})", expr, self.packet),
    }
  }
}
