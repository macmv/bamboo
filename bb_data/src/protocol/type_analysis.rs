use super::{convert, Cond, Expr, Field, Instr, Lit, Op, Packet, RType, Type, Value, VarKind};

#[derive(Debug)]
struct ReaderTypes<'a> {
  var_types: Vec<RType>,
  fields:    &'a mut Vec<Field>,
  packet:    &'a str,

  // For writer gen.
  //
  // The bool is `true` if the variable is used.
  vars: Vec<(bool, Expr)>,

  // Stores a read instruction that needs to be inverted. For example:
  // ```
  // let v_2 = p.read_i8();
  // f_invulnerable = if v_2 & 1 != 0 { 1 } else { 0 } != 0;
  // f_flying = if v_2 & 2 != 0 { 1 } else { 0 } != 0;
  // f_allow_flying = if v_2 & 4 != 0 { 1 } else { 0 } != 0;
  // f_creative_mode = if v_2 & 8 != 0 { 1 } else { 0 } != 0;
  // ```
  //
  // When we read the `let` call, we store that `read_i8()` call in `need_to_write`. Then, once we
  // write another field, we reassemble that `v_2` in reverse. This is making the assumption that
  // we have completed the variable `v_2` by the time we write the next instruction, which is not
  // always true.
  var_to_write:      Option<usize>,
  // Used for an initial value other than 0. Only used in JoinGame packet.
  var_init_to_write: Option<Expr>,
  // Used when we write the variable into the buffer, at the end of the needs_to_write block.
  var_func_to_write: Option<String>,
  // For simpler cases, the above sometimes is invalid. This is when the variable defined in
  // `need_to_write` is never used. In these cases, we give up, and store that in this value.
  needs_to_write:    Vec<Instr>,
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
    r.simplify_conditionals(&mut self.reader.block);
    r.gen_writer(&self.reader.block, &mut self.writer.block);
  }
}
impl<'a> ReaderTypes<'a> {
  pub fn new(vars: &[VarKind], fields: &'a mut Vec<Field>, name: &'a str) -> Self {
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
      vars: vars.iter().map(|_| (false, Expr::new(Value::Null))).collect(),
      packet: name,
      var_to_write: None,
      var_init_to_write: None,
      var_func_to_write: None,
      needs_to_write: vec![],
    }
  }
  fn get_field(&self, name: &str) -> Option<&Field> {
    self.fields.iter().find(|field| field.name == name)
  }
  fn get_field_mut(&mut self, name: &str) -> Option<&mut Field> {
    self.fields.iter_mut().find(|field| field.name == name)
  }
  fn use_expr(&mut self, expr: &Expr) {
    if let Value::Var(v) = expr.initial {
      if expr.initial != Value::packet_var() {
        self.vars[v].0 = true;
      }
    }
  }
  fn find_instr(&mut self, instr: &[Instr]) {
    for i in instr {
      match i {
        Instr::Set(field, expr) => {
          self.use_expr(expr);
          let ty = self.expr_type(expr);
          if let Some(f) = self.get_field_mut(field) {
            let rs_ty = f.ty.to_rust();
            if rs_ty.name == "U" {
              f.ty = Type::Rust(ty.clone());
            }
            if f.reader_type.is_none() {
              f.reader_type = Some(ty);
            }
          }
        }
        Instr::SetArr(_arr, _idx, _val) => {}
        Instr::Let(v, expr) => self.var_types[*v] = self.expr_type(expr),
        Instr::If(_cond, when_true, when_false) => {
          self.find_instr(when_true);
          self.find_instr(when_false);
        }
        Instr::For(_v, _range, _block) => {}
        Instr::Switch(_v, _tab, _def) => {}
        Instr::CheckStrLen(_val, _len) => {}
        Instr::Expr(_v) => {}
        Instr::Return(_v) => {}
        _ => todo!("find types for instr {:?} on packet {}", i, self.packet),
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
        ("GameMode", "from_id") => RType::new("GameMode"),
        _ => {
          println!("need to find type for static call: {}::{}", class, name);
          RType::new("i32")
        }
      },
      Value::Static(_class, _name) => RType::new("U"),
      Value::Field(name) => {
        self.get_field(name).map(|v| v.ty.to_rust()).unwrap_or_else(|| RType::new("U"))
      }
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
      Value::Cond(_) => RType::new("bool"),
    }
  }
  fn var_type(&self, var: usize) -> RType {
    self.var_types.get(var).cloned().unwrap_or_else(|| RType::new("tcp::Packet"))
  }

  fn op_type(&mut self, initial: RType, op: &Op) -> RType {
    match op {
      Op::Call(class, name, args) => match class.as_str() {
        "tcp::Packet" => {
          // This should be valid, but `initial` is a "Pos" sometimes, and I couldn't be
          // bothered to fix it.
          /* assert_eq!(initial, RType::new("tcp::Packet")); */
          self.buffer_call(name, args)
        }
        "HashMap<i32, U>" => match name.as_str() {
          "get" => RType::new("U"),
          _ => todo!("call {}::{}({:?})", class, name, args),
        },
        "Supplier" => initial,
        "ParticleS2CPacket" => RType::new("ParticleData"),
        "Option" => match name.as_str() {
          "is_some" => RType::new("bool"),
          "as_ref" => initial,
          // Quirk: fields that are options have an `option` field set, but their type is not
          // `Option`. So, if we aren't unwrapping an `Option`, we need to return the initial
          // value.
          "unwrap" => {
            if initial.name == "Option" {
              initial.generics[0].clone()
            } else {
              initial
            }
          }
          _ => todo!("call {}::{}({:?})", class, name, args),
        },
        _ => todo!("call {}::{}({:?})", class, name, args),
      },
      Op::Cast(ty) => ty.to_rust(),
      Op::As(ty) => ty.clone(),
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
      // We just assume all reader functions return a T (instead of Result<T>), so this won't change
      // the type.
      Op::Try => initial,
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
      "remaining" => RType::new("usize"),
      _ => todo!("call `{name}`"),
    }
  }

  fn check_var_to_write(&mut self, writer: &mut Vec<Instr>) {
    if let Some(var) = self.var_to_write.take() {
      // If need_to_write_used is false, we still want to `take()` need_to_write.
      if self.needs_to_write.is_empty() {
        self.needs_to_write.clear();
      } else {
        // If not present, the variable isn't defined, so it is unused.
        if let Some(expr) = self.var_init_to_write.take() {
          writer.push(Instr::Let(var, expr));
        }
        for i in self.needs_to_write.drain(..) {
          writer.push(i);
        }
        writer.push(Instr::Expr(Expr::new(Value::packet_var()).op(Op::Call(
          "tcp::Packet".into(),
          self.var_func_to_write.take().unwrap(),
          vec![Expr::new(Value::Var(var))],
        ))));
      }
    } else if !self.needs_to_write.is_empty() {
      for i in self.needs_to_write.drain(..) {
        writer.push(i);
      }
    }
  }

  fn gen_writer(&mut self, read: &[Instr], writer: &mut Vec<Instr>) {
    let mut if_chain = None;
    for (i, instr) in read.iter().enumerate() {
      match instr {
        Instr::Set(field, expr) => {
          if self.changes_buf(expr) {
            self.check_var_to_write(writer);
            let mut field_val = Expr::new(Value::Field(field.into()));
            let field = self.get_field(field).unwrap();
            if field.option {
              if field.ty.to_rust().is_copy() {
                field_val.add_op(Op::Call("Option".into(), "unwrap".into(), vec![]));
              } else {
                field_val.add_op(Op::Call("Option".into(), "as_ref".into(), vec![]));
                field_val.add_op(Op::Call("Option".into(), "unwrap".into(), vec![]));
              }
            }
            if let Some(instr) = self.set_expr(expr, &field_val) {
              writer.push(instr);
            }
          } else if let Some(i) = self.set_expr(expr, &Expr::new(Value::Field(field.clone()))) {
            self.needs_to_write.push(i);
          }
        }
        Instr::Let(i, expr) => {
          self.vars[*i] = (false, expr.clone());
          if let Some(func) = self.reverse_set(expr) {
            self.var_to_write = Some(*i);
            self.var_func_to_write = Some(func);
          }
        }
        Instr::Return(_) => {}
        Instr::For(_, _range, _) => {}
        Instr::Switch(_, _table, _def) => {}
        Instr::If(cond, when_true, when_false) => {
          let mut when_t = vec![];
          let mut when_f = vec![];
          self.gen_writer(when_true, &mut when_t);
          self.gen_writer(when_false, &mut when_f);

          // If we check against local variables, we need to assume the conditional based
          // on which fields were changed. Otherwise, we can just copy the conditional to
          // the writer.
          let mut needs_assume = false;
          for expr in cond.all_exprs() {
            if let Value::Var(_) = expr.initial {
              needs_assume = true;
              break;
            }
          }
          if needs_assume || if_chain.is_some() {
            let fields_changed: Vec<_> = when_true
              .iter()
              .filter_map(|i| match i {
                Instr::Set(field, _) => Some(field),
                _ => None,
              })
              .collect();
            assert!(
              !fields_changed.is_empty(),
              "cannot have a conditional where no fields are modified"
            );

            let mut initial_var = None;
            if let Cond::Greater(lhs, rhs) | Cond::Neq(lhs, rhs) = cond {
              if if_chain.is_none() {
                match lhs.initial {
                  Value::Var(id) => {
                    if_chain = Some((id, vec![]));
                    initial_var = Some(id);
                  }
                  _ => unimplemented!(),
                }
              }

              assert_eq!(rhs, &Expr::new(Value::Lit(Lit::Int(0))));
              match lhs.ops.get(0) {
                // `if v_2 & 1 != 0 {}`
                Some(Op::BitAnd(and)) => {
                  assert_eq!(rhs.clone().unwrap_int(), 0);
                  assert!(and.clone().unwrap_int() > 0);
                  if let Some(var) = initial_var {
                    writer.push(Instr::Let(var, Expr::new(Value::Lit(Lit::Int(0)))));
                  }
                  writer.push(Instr::SetVarOr(
                    if_chain.as_ref().unwrap().0,
                    and.clone().op(Op::If(
                      Box::new(Cond::Bool(
                        Expr::new(Value::Field(fields_changed[0].clone())).op(Op::Call(
                          "Option".into(),
                          "is_some".into(),
                          vec![],
                        )),
                      )),
                      Expr::new(Value::Lit(Lit::Int(0))),
                    )),
                  ));
                }
                // `if p.read_i32() != 0 {}`
                Some(Op::Call(..)) => {
                  assert_eq!(rhs.clone().unwrap_int(), 0);
                  writer.push(
                    self
                      .set_expr(
                        lhs,
                        &Expr::new(Value::Field(fields_changed[0].clone())).op(Op::Call(
                          "Option".into(),
                          "is_some".into(),
                          vec![],
                        )),
                      )
                      .unwrap(),
                  );
                }
                // `if v_2 != 0 {}`
                None => {
                  assert_eq!(rhs.clone().unwrap_int(), 0);
                  if let Some(var) = initial_var {
                    writer.push(Instr::Let(var, Expr::new(Value::Lit(Lit::Int(0)))));
                  }
                  writer.push(Instr::SetVar(
                    if_chain.as_ref().unwrap().0,
                    Expr::new(Value::Lit(Lit::Int(1))).clone().op(Op::If(
                      Box::new(Cond::Bool(
                        Expr::new(Value::Field(fields_changed[0].clone())).op(Op::Call(
                          "Option".into(),
                          "is_some".into(),
                          vec![],
                        )),
                      )),
                      Expr::new(Value::Lit(Lit::Int(0))),
                    )),
                  ));
                }
                v => unimplemented!("lhs {v:?}, rhs {rhs:?}"),
              }
            } else {
              unimplemented!();
            }

            if_chain.as_mut().unwrap().1.push(Instr::If(
              Cond::Bool(Expr::new(Value::Field(fields_changed[0].clone())).op(Op::Call(
                "Option".into(),
                "is_some".into(),
                vec![],
              ))),
              when_t,
              when_f,
            ));
            if matches!(read.get(i + 1), Some(Instr::If(_, _, _))) {
              continue;
            } else {
              let (var, mut updates) = if_chain.take().unwrap();
              if Value::Var(var) == Value::packet_var() || var == 1 {
                // if p.read_i32() != 0 { ... }
                for instr in updates.drain(..) {
                  writer.push(instr);
                }
              } else {
                // This is the initial value we read and compared against:
                // let v_2 = p.read_i32();
                //           ^^^^^^^^^^^^ this expression
                // if v_2 & 1 != 0 { ... }
                let initial_read = self.value_of(&Expr::new(Value::Var(var)));
                writer.push(
                  self
                    .set_expr(&initial_read, &Expr::new(Value::Var(var)))
                    .unwrap_or_else(|| panic!("could not create writer from {initial_read:?}")),
                );
                for instr in updates.drain(..) {
                  writer.push(instr);
                }
              }
            }
          } else {
            writer.push(Instr::If(cond.clone(), when_t, when_f));
          }
        }
        _ => panic!("cannot convert {:?} into writer", i),
      }
    }
    self.check_var_to_write(writer);
  }

  fn changes_buf(&self, expr: &Expr) -> bool {
    matches!(
      expr.ops.first(),
      Some(Op::Call(class, name, _args)) if class == "tcp::Packet" && name != "remaining"
    )
  }

  fn set_expr(&mut self, expr: &Expr, field: &Expr) -> Option<Instr> {
    match &expr.initial {
      Value::CallStatic(class, name, args) => {
        let new_name;
        let mut new_args = vec![];
        match (class.as_str(), name.as_str()) {
          ("GameMode", "from_id") => {
            new_name = "write_u8".into();
            new_args.push(args[0].clone().op(Op::Call("GameMode".into(), "id".into(), vec![])))
          }
          _ => return None,
        };
        return Some(Instr::Expr(Expr::new(Value::packet_var()).op(Op::Call(
          "tcp::Packet".into(),
          new_name,
          new_args,
        ))));
      }
      Value::Var(v) => {
        if self.var_to_write.map(|var| var == *v) == Some(true) && expr.ops.is_empty() {
          self.var_init_to_write = Some(field.clone());
          return None;
        }
      }
      Value::Cond(cond) => {
        if let Some(var_to_write) = self.var_to_write {
          // self.needs_to_write.push(Instr::Expr(expr.clone()));
          let (lhs, inverted) = match cond.as_ref() {
            Cond::Greater(lhs, rhs) => {
              assert_eq!(rhs, &Expr::new(Value::Lit(0.into())));
              (lhs, false)
            }
            // Right now, we assume that we have a BitAnd like so: `v & 8 != 0` or `v & 8 == 8`.
            // This is easy to check with eq/neq to 0, but fails if we have something like this: `v
            // & 8 != 5`. This last check makes no sense, and doesn't appear in our use case, so I
            // am going to ignore it.
            Cond::Neq(lhs, rhs) => (lhs, rhs != &Expr::new(Value::Lit(0.into()))),
            Cond::Eq(lhs, rhs) => (lhs, rhs == &Expr::new(Value::Lit(0.into()))),
            _ => unimplemented!("cond {:?}", cond),
          };
          let mut cond = field.clone();
          if inverted {
            cond.add_op(Op::Not);
          }
          return Some(Instr::If(
            Cond::Bool(cond),
            vec![match lhs.ops.first() {
              Some(Op::BitAnd(rhs)) => {
                let lhs = &lhs.initial;
                let rhs = &rhs.initial;
                assert_eq!(lhs, &Value::Var(var_to_write));
                Instr::SetVar(
                  var_to_write,
                  Expr::new(Value::Var(var_to_write)).op(Op::BitOr(Expr::new(rhs.clone()))),
                )
              }
              _ => unimplemented!(),
            }],
            vec![],
          ));
        } else {
          match cond.as_ref() {
            // This is where we have a reader like so:
            // f_foo = p.read_u8() != 0;
            //
            // So we generate this writer:
            // p.write_bool(f_foo);
            //
            // This only happens on old versions, where they didn't use the `read_bool` function at
            // all. This might end up being removed if I simplify all the `!= 0` calls into
            // `read_bool`.
            Cond::Neq(lhs, rhs) => {
              assert_eq!(rhs, &Expr::new(Value::Lit(0.into())));
              match lhs.ops.first().unwrap() {
                Op::Call(class, _name, _args) if class == "tcp::Packet" => {}
                _ => unimplemented!(),
              };
              return Some(Instr::Expr(Expr::new(Value::packet_var()).op(Op::Call(
                "tcp::Packet".into(),
                "write_bool".into(),
                vec![field.clone()],
              ))));
            }
            _ => unimplemented!(),
          }
        }
      }
      _ => {}
    }
    Some(match expr.ops.first() {
      Some(Op::Call(class, name, _args)) if class == "tcp::Packet" => {
        assert!(expr.initial.is_packet_var(), "unknown Set value: {:?}", expr);
        let writer_name = convert::reader_to_writer(name);
        let mut val = field.clone();
        for op in expr.ops.iter().skip(1).rev() {
          val.ops.push(match op {
            // Convert the cast `foo = buf.read_u8() as i32` into `buf.write_u8(foo as u8)`
            Op::Cast(_from) => {
              let mut e = expr.clone();
              e.ops.drain(1..val.ops.len() + 2);
              Op::As(self.expr_type(&e))
            }
            Op::BitAnd(v) => Op::BitAnd(v.clone()),
            Op::Div(v) => Op::Mul(v.clone()),
            // When converting from reader to writer, we remove the `?` from the ops.
            Op::Try => continue,
            _ => panic!("cannot convert {:?} into writer (packet {})", expr, self.packet),
          });
        }
        Instr::Expr(Expr::new(Value::packet_var()).op(Op::Call(
          class.clone(),
          writer_name.into(),
          vec![self.writer_cast(val, convert::reader_func_to_ty("", name))],
        )))
      }
      Some(_) => return None,
      None => return None,
      // _ => panic!("cannot convert {:?} into writer (packet {})", expr, self.packet),
    })
  }

  fn value_of(&self, v: &Expr) -> Expr {
    match v.initial {
      Value::Var(idx) if idx != 1 => self.vars[idx].1.clone(),
      _ => v.clone(),
    }
  }

  fn writer_cast(&mut self, mut expr: Expr, field_ty: RType) -> Expr {
    let writer_ty = self.expr_type(&expr);
    if writer_ty != field_ty {
      expr.ops.extend(convert::type_cast(&writer_ty, &field_ty));
    }
    // Remove double cast, which causes the below deref check to work in more
    // situations.
    if matches!(expr.ops.last(), Some(Op::As(_)))
      && matches!(expr.ops.iter().rev().nth(1), Some(Op::As(_)))
    {
      let len = expr.ops.len();
      expr.ops.remove(len - 2);
    }
    if !field_ty.is_copy() {
      expr.ops.insert(0, Op::Ref);
    }
    expr
  }

  fn reverse_set(&self, expr: &Expr) -> Option<String> {
    match expr.ops.first() {
      Some(Op::Call(class, name, _args)) if class == "tcp::Packet" => {
        Some(convert::reader_to_writer(name).into())
      }
      _ => None,
    }
  }

  fn simplify_conditionals(&mut self, instr: &mut [Instr]) {
    let mut assigned_and_unused = vec![None; self.vars.len()];
    for i in instr {
      match i {
        Instr::Set(name, expr) => {
          if let Some(field) = self.get_field_mut(name) {
            if field.reader_type == Some(RType::new("i32")) {
              if let Some(Op::If(cond, _)) = expr.ops.last() {
                // expr.ops.pop();
                field.reader_type = Some(RType::new("bool"));
                *expr = Expr::new(Value::Cond(cond.clone()));
                // expr = Expr::Cond(cond);
              }
            }
          }
        }
        Instr::Let(var, expr) => {
          if let Some(Op::If(cond, _)) = expr.ops.last() {
            // expr.ops.pop();
            self.vars[*var].1 = Expr::new(Value::Cond(cond.clone()));
            *expr = Expr::new(Value::Cond(cond.clone()));
            // expr = Expr::Cond(cond);
          }
          if !self.vars[*var].0 {
            assigned_and_unused[*var] = Some(&*expr);
          }
        }
        _ => {}
      }
    }
    for (var, expr) in assigned_and_unused.into_iter().enumerate() {
      if let Some(e) = expr {
        let rty = self.expr_type(e);
        self.fields.push(Field {
          name:        format!("v_{var}"),
          ty:          Type::Void,
          reader_type: Some(rty),
          option:      false,
          initialized: true,
        });
      }
    }
  }
}
