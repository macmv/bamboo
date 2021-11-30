use super::{Expr, Field, Instr, Lit, Packet, RType, Value, VarKind};

#[derive(Debug)]
struct ReaderTypes<'a> {
  var_types: Vec<RType>,
  fields:    &'a mut [Field],
}

impl Packet {
  pub fn find_reader_types(&mut self) {
    let mut var_types = Vec::with_capacity(self.reader.vars.len());
    for v in &self.reader.vars {
      match v {
        VarKind::This => var_types.push(RType::new("Self")),
        VarKind::Arg => var_types.push(RType::new("tcp::Packet")),
        VarKind::Local => var_types.push(RType::new("U")),
      }
    }
    let mut r = ReaderTypes { var_types, fields: &mut self.fields };
    r.find_instr(&self.reader.block);
  }
}
impl ReaderTypes<'_> {
  #[track_caller]
  fn get_field(&self, name: &str) -> &Field {
    self.fields.iter().find(|field| field.name == name).unwrap()
  }
  #[track_caller]
  fn get_field_mut(&mut self, name: &str) -> &mut Field {
    self.fields.iter_mut().find(|field| field.name == name).unwrap()
  }
  fn find_instr(&mut self, instr: &[Instr]) {
    for i in instr {
      match i {
        Instr::Set(field, expr) => {
          self.get_field_mut(field).reader_type = Some(self.expr_type(expr))
        }
        Instr::SetArr(arr, idx, val) => {}
        Instr::Let(v, val) => {}
        Instr::If(cond, when_true, when_false) => {}
        Instr::For(v, range, block) => {}
        Instr::Switch(v, tab) => {}
        Instr::CheckStrLen(val, len) => {}
        Instr::Expr(v) => {}
        Instr::Return(v) => {}
        _ => todo!(),
      }
    }
  }

  fn expr_type(&self, expr: &Expr) -> RType {
    let initial = self.val_type(&expr.initial);
    initial
  }

  fn val_type(&self, val: &Value) -> RType {
    match val {
      Value::Lit(v) => match v {
        Lit::Int(_) => RType::new("i32"),
        Lit::Float(_) => RType::new("f32"),
        Lit::String(_) => RType::new("String"),
      },
      Value::Var(v) => self.var_type(*v),
      Value::CallStatic(class, name, _args) => match (class.as_str(), name.as_str()) {
        ("HashMap", "new") => RType::new("HashMap"),
        ("HashSet", "new") => RType::new("HashSet"),
        _ => {
          println!("need to find type for static call: {}::{}", class, name);
          RType::new("i32")
        }
      },
      Value::Static(class, name) => RType::new("i32"),
      Value::Field(name) => {
        dbg!(&self, &name);
        self.get_field(name).reader_type.clone().unwrap()
      }
      _ => todo!("value: {:?}", val),
    }
  }

  fn var_type(&self, var: usize) -> RType {
    self.var_types[var].clone()
  }
}
