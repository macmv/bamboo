use super::{convert, Cond, Expr, Field, Instr, Lit, Op, Packet, RType, Type, Value, VarKind};

#[derive(Debug)]
struct ReaderTypes<'a> {
  var_types: Vec<RType>,
  fields:    &'a mut [Field],
  packet:    &'a str,

  // For writer gen.
  vars: Vec<Expr>,
}

impl Packet {
  /// Finds all the reader types, then generates the `writer` function. This is
  /// all in the same function because finding the reader types also involves
  /// finding all the local variable types. The `writer` function is much easier
  /// to generate if we already know all of that information.
  pub fn find_reader_types_gen_writer(&mut self) {
    for f in &mut self.fields {
      if f.ty.to_rust().name == "tcp::Packet" {
        f.ty = Type::Class("U".into());
      }
    }
    let mut r = ReaderTypes::new(&self.reader.vars, &mut self.fields, &self.name);
    r.find_instr(&self.reader.block);
    r.gen_writer(&self.reader.block, &mut self.writer.block);
  }
}
impl<'a> ReaderTypes<'a> {
  pub fn new(vars: &[VarKind], fields: &'a mut [Field], name: &'a str) -> Self {
    let mut var_types = Vec::with_capacity(vars.len());
    for v in vars {
      match v {
        VarKind::This => var_types.push(RType::new("Self")),
        VarKind::Arg => var_types.push(RType::new("tcp::Packet")),
        VarKind::Local => var_types.push(RType::new("U")),
      }
    }
    ReaderTypes {
      var_types,
      fields,
      vars: vars.iter().map(|_| Expr::new(Value::Null)).collect(),
      packet: name,
    }
  }
  fn get_field(&self, name: &str) -> Option<&Field> {
    self.fields.iter().find(|field| field.name == name)
  }
  fn get_field_mut(&mut self, name: &str) -> Option<&mut Field> {
    self.fields.iter_mut().find(|field| field.name == name)
  }
  fn find_instr(&mut self, instr: &[Instr]) {
    for i in instr {
      match i {
        Instr::Set(field, expr) => {
          let ty = self.expr_type(expr);
          self.get_field_mut(field).map(|f| {
            let rs_ty = f.ty.to_rust();
            if rs_ty.name == "U" {
              f.ty = Type::Rust(ty.clone());
            }
            f.reader_type = Some(ty);
          });
        }
        Instr::SetArr(_arr, _idx, _val) => {}
        Instr::Let(_v, _val) => {}
        Instr::If(_cond, _when_true, _when_false) => {}
        Instr::For(_v, _range, _block) => {}
        Instr::Switch(_v, _tab) => {}
        Instr::CheckStrLen(_val, _len) => {}
        Instr::Expr(_v) => {}
        Instr::Return(_v) => {}
        _ => todo!(),
      }
    }
  }

  fn expr_type(&mut self, expr: &Expr) -> RType {
    let mut ty = self.val_type(&expr.initial);
    for op in &expr.ops {
      ty = self.op_type(ty, op);
    }
    ty
  }

  fn val_type(&mut self, val: &Value) -> RType {
    match val {
      Value::Lit(v) => match v {
        Lit::Int(_) => RType::new("i32"),
        Lit::Float(_) => RType::new("f32"),
        Lit::String(_) => RType::new("String"),
      },
      Value::Null => RType::new("Option").generic("U"),
      Value::Var(v) => self.var_type(*v),
      Value::CallStatic(class, name, _args) => match (class.as_str(), name.as_str()) {
        ("HashMap", "new") => RType::new("HashMap"),
        ("HashSet", "new") => RType::new("HashSet"),
        _ => {
          println!("need to find type for static call: {}::{}", class, name);
          RType::new("i32")
        }
      },
      Value::Static(_class, _name) => RType::new("U"),
      Value::Field(name) => self
        .get_field(name)
        .map(|v| v.reader_type.clone().unwrap_or(RType::new("U")))
        .unwrap_or(RType::new("U")),
      Value::New(_class, _args) => RType::new("U"),
      Value::Array(_) => RType::new("Vec"),
      Value::MethodRef(class, name) => match class.as_str() {
        "tcp::Packet" => self.buffer_call(name, &[]),
        "AdvancementTask" => match name.as_str() {
          "from_packet" => RType::new("AdvancementTask"),
          _ => todo!("static ref {}::{}", class, name),
        },
        "AdvancementProgress" => match name.as_str() {
          "from_packet" => RType::new("AdvancementProgress"),
          _ => todo!("static ref {}::{}", class, name),
        },
        "HashMap" => match name.as_str() {
          "new" | "with_capacity" => RType::new("HashMap").generic("U").generic("U"),
          _ => todo!("static ref {}::{}", class, name),
        },
        "Object2IntOpenHashMap" => match name.as_str() {
          "<init>" => RType::new("HashMap").generic("U").generic("i32"),
          _ => todo!("static ref {}::{}", class, name),
        },
        "HashSet" => match name.as_str() {
          "new" | "with_capacity" => RType::new("HashSet").generic("U"),
          _ => todo!("static ref {}::{}", class, name),
        },
        "PlayerListS2CPacket$Action" => RType::new("PlayerListAction"),
        "SynchronizeRecipesS2CPacket" => RType::new("Recipe"),
        "TagGroup$Serialized" => RType::new("TabGroup"),
        _ => todo!("static ref {}::{}", class, name),
      },
      Value::Closure(_, block) => {
        let mut r = ReaderTypes::new(&block.vars, self.fields, self.packet);
        r.find_instr(&block.block);
        r.expr_type(match block.block.last().unwrap() {
          Instr::Return(v) => v,
          _ => unreachable!(),
        })
      }
    }
  }
  fn var_type(&self, var: usize) -> RType {
    self.var_types[var].clone()
  }

  fn op_type(&mut self, initial: RType, op: &Op) -> RType {
    match op {
      Op::Call(class, name, args) => match class.as_str() {
        "tcp::Packet" => {
          assert_eq!(initial, RType::new("tcp::Packet"));
          self.buffer_call(name, args)
        }
        "HashMap<i32, U>" => match name.as_str() {
          "get" => RType::new("U"),
          _ => todo!("call {}::{}({:?})", class, name, args),
        },
        "Supplier" => initial,
        "ParticleS2CPacket" => RType::new("ParticleData"),
        _ => todo!("call {}::{}({:?})", class, name, args),
      },
      Op::Cast(ty) => ty.to_rust(),
      Op::If(_cond, new) => {
        // TODO: When we get an Option<T> and T, we need to wrap T in Some().
        let new_ty = self.expr_type(new);
        if new_ty.name == "Option" {
          new_ty
        } else if initial.name == "Option" {
          initial
        } else {
          assert_eq!(initial, new_ty);
          initial
        }
      }
      Op::BitAnd(_) | Op::Add(_) | Op::Sub(_) | Op::Div(_) | Op::Mul(_) => initial,
      v => todo!("op {:?}", v),
    }
  }

  fn buffer_call(&mut self, name: &str, args: &[Expr]) -> RType {
    match name {
      "read_varint" => RType::new("i32"),
      "read_u8" => RType::new("u8"),
      "read_i8" => RType::new("i8"),
      "read_i16" => RType::new("i16"),
      "read_i32" => RType::new("i32"),
      "read_i64" => RType::new("i64"),
      "read_f32" => RType::new("f32"),
      "read_f64" => RType::new("f64"),
      "read_pos" => RType::new("Pos"),
      "read_str" | "read_ident" => RType::new("String"),
      "read_uuid" => RType::new("UUID"),
      "read_byte_arr" | "read_all" => RType::new("Vec").generic("u8"),
      "read_i32_arr" => RType::new("Vec").generic("i32"),
      "read_varint_arr" => RType::new("Vec").generic("i32"),
      "read_block_hit" => RType::new("BlockHit"),
      "read_nbt" => RType::new("NBT"),
      "read_item" => RType::new("Item"),
      "read_bits" => RType::new("BitSet"),
      "read_map" => {
        RType::new("HashMap").generic(self.expr_type(&args[0])).generic(self.expr_type(&args[1]))
      }
      "read_list" => RType::new("Vec").generic(self.expr_type(&args[0])),
      _ => todo!("call {}", name),
    }
  }

  fn gen_writer(&mut self, read: &[Instr], writer: &mut Vec<Instr>) {
    for i in read {
      match i {
        Instr::Set(field, expr) => {
          if let Some(instr) = self.set_expr(expr, &Expr::new(Value::Field(field.into()))) {
            writer.push(instr);
          }
        }
        Instr::Let(i, expr) => self.vars[*i] = expr.clone(),
        Instr::Return(_) => {}
        Instr::For(_, _range, _) => {}
        Instr::Switch(_, _table) => {}
        Instr::If(cond, when_true, when_false) => {
          // let mut when_t = vec![];
          // let mut when_f = vec![];
          // self.gen_writer(when_true, &mut when_t);
          // self.gen_writer(when_false, &mut when_f);

          let fields_changed: Vec<_> = when_true
            .iter()
            .filter_map(|i| match i {
              Instr::Set(field, _) => Some(field),
              _ => None,
            })
            .collect();
          assert!(
            fields_changed.len() > 0,
            "cannot have a conditional where no fields are modified"
          );

          match cond {
            Cond::Neq(lhs, rhs) => {
              assert_eq!(rhs, &Expr::new(Value::Lit(Lit::Int(0))));
              let v = self.value_of(lhs);
              writer.push(
                self
                  .set_expr(
                    &v,
                    &Expr::new(Value::Field(fields_changed[0].clone())).op(Op::Call(
                      "Option".into(),
                      "is_some".into(),
                      vec![],
                    )),
                  )
                  .unwrap(),
              );
            }
            _ => {
              writer.push(Instr::If(cond.clone(), when_true.clone(), when_false.clone()));
            } // _ => todo!("cond {:?}", cond),
          }

          // writer.push(Instr::If(cond.clone(), when_t, when_f));
        }
        _ => panic!("cannot convert {:?} into writer", i),
      }
    }
  }

  fn set_expr(&mut self, expr: &Expr, field: &Expr) -> Option<Instr> {
    Some(match expr.ops.first() {
      Some(Op::Call(class, name, _args)) if class == "tcp::Packet" => {
        assert_eq!(expr.initial, Value::Var(1), "unknown Set value: {:?}", expr);
        let writer_name = convert::reader_to_writer(name);
        let mut val = field.clone();
        for op in expr.ops.iter().skip(1).rev() {
          val.ops.push(match op {
            // Convert the cast `foo = buf.read_u8() as i32` into `buf.write_u8(foo as u8)`
            Op::Cast(_from) => {
              let mut e = expr.clone();
              e.ops.drain(1..val.ops.len() + 1);
              Op::As(self.expr_type(&e))
            }
            Op::BitAnd(v) => Op::BitAnd(v.clone()),
            Op::Div(v) => Op::Mul(v.clone()),
            _ => panic!("cannot convert {:?} into writer (packet {})", expr, self.packet),
          });
        }
        Instr::Expr(Expr::new(Value::Var(1)).op(Op::Call(
          class.clone(),
          writer_name.into(),
          vec![val],
        )))
      }
      Some(Op::If(_cond, _new)) => return None,
      None => return None,
      _ => panic!("cannot convert {:?} into writer (packet {})", expr, self.packet),
    })
  }

  fn value_of(&self, v: &Expr) -> Expr {
    match v.initial {
      Value::Var(idx) if idx != 1 => self.vars[idx].clone(),
      _ => v.clone(),
    }
  }
}
