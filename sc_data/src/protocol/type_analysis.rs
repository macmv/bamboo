use super::{Expr, Field, Instr, Lit, Op, Packet, RType, Value, VarKind};

#[derive(Debug)]
struct ReaderTypes<'a> {
  var_types: Vec<RType>,
  fields:    &'a mut [Field],
}

impl Packet {
  pub fn find_reader_types(&mut self) {
    let mut r = ReaderTypes::new(&self.reader.vars, &mut self.fields);
    r.find_instr(&self.reader.block);
  }
}
impl<'a> ReaderTypes<'a> {
  pub fn new(vars: &[VarKind], fields: &'a mut [Field]) -> Self {
    let mut var_types = Vec::with_capacity(vars.len());
    for v in vars {
      match v {
        VarKind::This => var_types.push(RType::new("Self")),
        VarKind::Arg => var_types.push(RType::new("tcp::Packet")),
        VarKind::Local => var_types.push(RType::new("U")),
      }
    }
    ReaderTypes { var_types, fields }
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
          self.get_field_mut(field).map(|v| v.reader_type = Some(ty));
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
        let mut r = ReaderTypes::new(&block.vars, self.fields);
        r.find_instr(&block.block);
        r.expr_type(match block.block.last().unwrap() {
          Instr::Return(v) => v,
          _ => unreachable!(),
        })
      }
      _ => todo!("value: {:?}", val),
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
      // TODO: Math ops should coerce types
      Op::BitAnd(_) | Op::Div(_) => initial,
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
}
