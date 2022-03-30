use super::{convert, Cond, Expr, Field, Instr, Lit, Op, Packet, RType, Type, Value};
use convert_case::{Case, Casing};
use std::mem;

// Called before extend_from
pub fn pass(p: &mut Packet) {
  let len = simplify_instr(&mut p.reader.block);
  if let Some(l) = len {
    p.reader.block = p.reader.block[..l].to_vec();
    p.fields.push(Field {
      name:        "unknown".into(),
      ty:          Type::Rust(RType::new("Vec").generic("u8")),
      reader_type: None,
      option:      false,
      initialized: false,
    });
  }

  for f in &mut p.fields {
    simplify_name(&mut f.name);
  }
}
// Called after extend_from
pub fn finish(p: &mut Packet) {
  if (p.reader.block.len() == 1 && matches!(p.reader.block.last(), Some(Instr::Return(_))))
    || p.fields.is_empty()
  {
    p.reader.block.clear();
    p.reader.block.push(set_unknown());
    p.fields.push(Field {
      name:        "unknown".into(),
      ty:          Type::Rust(RType::new("Vec").generic("u8")),
      reader_type: None,
      option:      false,
      initialized: false,
    });
  }

  let fields = Vec::with_capacity(p.fields.len());
  let old_fields = mem::replace(&mut p.fields, fields);
  for mut f in old_fields {
    match init_state(&p.reader.block, &f.name) {
      // Don't push the field if it is not used
      InitState::Unused => {}
      InitState::Used { is_option, needs_default } => {
        if is_option && matches!(f.ty, Type::Array(_)) {
          f.option = false;
          f.initialized = false;
        } else {
          f.initialized = !needs_default;
          f.option = is_option;
        }
        p.fields.push(f);
      }
    }
  }
}
fn simplify_instr(instr: &mut [Instr]) -> Option<usize> {
  for (idx, i) in instr.iter_mut().enumerate() {
    match i {
      Instr::Super => {}
      Instr::Set(name, val) => {
        simplify_name(name);
        match simplify_expr_overwrite(val) {
          (false, Some(new_instr)) => *i = new_instr,
          (false, None) => {}
          (true, _) => {
            *i = set_unknown();
            return Some(idx + 1);
          }
        };
      }
      Instr::SetArr(arr, idx, val) => {
        simplify_expr(arr);
        let _ = simplify_val(idx);
        simplify_expr(val);
      }
      Instr::SetVar(_, val) | Instr::SetVarOr(_, val) | Instr::Let(_, val) => {
        match simplify_expr_overwrite(val) {
          (false, Some(new_instr)) => *i = new_instr,
          (false, None) => {}
          (true, _) => {
            *i = set_unknown();
            return Some(idx + 1);
          }
        };
      }
      Instr::Expr(v) => match simplify_expr_overwrite(v) {
        (false, Some(new_instr)) => *i = new_instr,
        (false, None) => {}
        (true, _) => {
          *i = set_unknown();
          return Some(idx + 1);
        }
      },
      Instr::If(cond, when_true, when_false) => match simplify_cond_overwrite(cond) {
        (false, Some(new_instr)) => *i = new_instr,
        (false, None) => {
          if simplify_instr(when_true).is_some() {
            *i = set_unknown();
            return Some(idx + 1);
          }
          if simplify_instr(when_false).is_some() {
            *i = set_unknown();
            return Some(idx + 1);
          }
        }
        (true, _) => {
          *i = set_unknown();
          return Some(idx + 1);
        }
      },
      Instr::For(_var, _range, _block) => {
        *i = set_unknown();
        return Some(idx + 1);
        // simplify_expr(&mut range.min);
        // simplify_expr(&mut range.max);
        // simplify_instr(block);
      }
      Instr::Switch(_val, _items) => {
        *i = set_unknown();
        return Some(idx + 1);
        // simplify_expr(val);
        // for (_, instr) in items {
        //   simplify_instr(instr);
        // }
      }
      Instr::CheckStrLen(s, len) => {
        simplify_expr(s);
        let _ = simplify_val(len);
      }
      Instr::Return(v) => simplify_expr(v),
    }
  }
  None
}
fn set_unknown() -> Instr {
  Instr::Set(
    "unknown".into(),
    Expr::new(Value::packet_var()).op(Op::Call("tcp::Packet".into(), "read_all".into(), vec![])),
  )
}
fn simplify_cond(cond: &mut Cond) { simplify_cond_overwrite(cond); }
fn simplify_cond_overwrite(cond: &mut Cond) -> (bool, Option<Instr>) {
  match cond {
    Cond::Eq(lhs, rhs)
    | Cond::Neq(lhs, rhs)
    | Cond::Less(lhs, rhs)
    | Cond::Greater(lhs, rhs)
    | Cond::Lte(lhs, rhs)
    | Cond::Gte(lhs, rhs) => {
      match simplify_expr_overwrite(lhs) {
        v @ (false, Some(_)) | v @ (true, _) => {
          simplify_expr(rhs);
          return v;
        }
        _ => {}
      }
      match simplify_expr_overwrite(rhs) {
        v @ (false, Some(_)) | v @ (true, _) => return v,
        _ => {}
      }
      (false, None)
    }
    Cond::Or(lhs, rhs) => {
      match simplify_cond_overwrite(lhs) {
        v @ (false, Some(_)) | v @ (true, _) => {
          simplify_cond(rhs);
          return v;
        }
        _ => {}
      }
      match simplify_cond_overwrite(rhs) {
        v @ (false, Some(_)) | v @ (true, _) => return v,
        _ => {}
      }
      (false, None)
    }
    Cond::Bool(val) => simplify_expr_overwrite(val),
  }
}
fn simplify_expr(expr: &mut Expr) { simplify_expr_overwrite(expr); }
fn simplify_expr_overwrite(expr: &mut Expr) -> (bool, Option<Instr>) {
  fn simplify(expr: &mut Expr) -> (bool, Option<Instr>) {
    expr.ops.extend(simplify_val(&mut expr.initial));
    let len = expr.ops.len();
    let old_ops = std::mem::replace(&mut expr.ops, Vec::with_capacity(len));
    for mut op in old_ops {
      let (skip, extra_ops) = simplify_op(&mut op);
      if skip {
        return (true, None);
      }
      expr.ops.push(op);
      expr.ops.extend(extra_ops);
    }
    (false, convert::overwrite(expr))
  }
  let res = match &expr.initial {
    Value::CallStatic(class, name, args) => {
      match (class.as_str(), name.as_str()) {
        ("net/minecraft/world/WorldSettings$GameType", "getByID") => *expr = args[0].clone(),
        ("net/minecraft/world/GameType", "getByID") => *expr = args[0].clone(),
        ("net/minecraft/world/EnumDifficulty", "getDifficultyEnum") => *expr = args[0].clone(),
        ("net/minecraft/world/WorldType", "parseWorldType") => *expr = args[0].clone(),
        _ => return (true, None),
      }
      simplify_expr(expr);
      return (false, None);
    }
    Value::New(..) | Value::Array(_) | Value::Field(..) => return (true, None),
    Value::Static(class, _name) => {
      expr.initial = match class.as_str() {
        "net/minecraft/entity/item/EntityPainting$EnumArt" => {
          Value::Lit(Lit::Int("SkullAndRoses".len() as i32))
        }
        _ => return (true, None),
      };
      return (true, None);
    }
    Value::Var(0) => match expr.ops.first_mut() {
      Some(Op::Call(class, name, args)) if class != "net/minecraft/network/PacketByteBuf" => {
        let instr = match convert::this_call(name, args) {
          Some(v) => v,
          None => return (true, None),
        };
        let mut arr = vec![instr];
        if simplify_instr(&mut arr).is_some() {
          return (true, None);
        } else {
          return (false, Some(arr.pop().unwrap()));
        }
      }
      _ => simplify(expr),
    },
    Value::Var(1) => match expr.ops.first() {
      Some(Op::Call(_class, name, _args)) => match name.as_str() {
        "readCollection" | "readList" | "readMap" => return (true, None),
        _ => {
          let res = simplify(expr);
          match expr.ops.first() {
            Some(Op::Call(_class, name, _args)) => match name.as_str() {
              "read_varint" | "read_i8" | "read_u8" | "read_i16" | "read_i32" | "read_i64"
              | "read_f32" | "read_f64" | "read_str" | "read_pos" | "read_uuid" | "remaining"
              | "read_varint_arr" | "read_i32_arr" => res,
              "read_item" | "read_block_hit" | "read_nbt" | "read_bits" => return (true, None),
              name => panic!("{}", name),
              // _ => return (true, None),
            },
            _ => return (true, None),
          }
        }
      },
      // _ => simplify(expr),
      _ => return (true, None),
    },
    _ => simplify(expr),
  };
  for o in &expr.ops {
    match o {
      Op::Call(class, _name, _args) if class == "U" => return (true, None),
      _ => {}
    }
  }
  res
}
#[must_use]
fn simplify_val(val: &mut Value) -> Vec<Op> {
  match val {
    Value::Field(name) => simplify_name(name),
    Value::Static(..) => {}
    Value::Array(len) => simplify_expr(len),
    Value::CallStatic(class, name, args) => {
      simplify_name(name);
      *class = convert::class(class).to_string();
      let (new_class, new_name) = convert::static_call(class, name);
      *class = new_class.into();
      *name = new_name.into();
      args.iter_mut().for_each(simplify_expr);
    }
    Value::MethodRef(class, name) => {
      simplify_name(name);
      *class = convert::class(class).to_string();
      *val = convert::static_ref(class, name);
    }
    Value::Closure(args, block) => {
      for a in args.iter_mut() {
        simplify_expr(a);
      }
      simplify_instr(&mut block.block);
    }
    Value::New(_, args) => {
      args.iter_mut().for_each(simplify_expr);
    }
    Value::Cond(cond) => simplify_cond(cond),
    Value::Null | Value::Lit(_) | Value::Var(_) => {}
  }
  vec![]
}
fn simplify_op(op: &mut Op) -> (bool, Vec<Op>) {
  match op {
    Op::BitAnd(rhs) => {
      simplify_expr(rhs);
      // For the join game packet.
      match rhs.initial {
        Value::Lit(Lit::Int(-9)) => {
          *rhs = Expr::new(Value::Lit(Lit::Int((-9i8 as u8).into())));
        }
        Value::Lit(Lit::Int(255)) => {
          *rhs = Expr::new(Value::Lit(Lit::Int((255u8 as i8).into())));
        }
        _ => {}
      }
    }
    Op::Shr(rhs) => simplify_expr(rhs),
    Op::UShr(rhs) => simplify_expr(rhs),
    Op::Shl(rhs) => simplify_expr(rhs),

    Op::Add(rhs) => simplify_expr(rhs),
    Op::Sub(rhs) => simplify_expr(rhs),
    Op::Div(rhs) => simplify_expr(rhs),
    Op::Mul(rhs) => simplify_expr(rhs),

    Op::Len => {}
    Op::Idx(idx) => simplify_expr(idx),
    Op::Field(_) => {}

    Op::If(cond, val) => {
      match simplify_cond_overwrite(cond) {
        (false, _) => {}
        (true, _) => return (true, vec![]),
      }
      match simplify_expr_overwrite(val) {
        (false, _) => {}
        (true, _) => return (true, vec![]),
      }
    }
    Op::Call(class, name, args) => {
      *class = convert::class(class).to_string();
      simplify_name(name);
      let (new_name, new_args, needs_try) = convert::member_call(class, name);
      *name = new_name.into();
      if let Some(a) = new_args {
        *args = a;
      } else {
        args.iter_mut().for_each(simplify_expr)
      }
      if needs_try {
        return (false, vec![Op::Try]);
      }
    }
    Op::Cast(_) => {}

    _ => unreachable!(),
  }
  (false, vec![])
}
fn simplify_name(name: &mut String) {
  if name == "type" {
    *name = "ty".into();
  } else {
    *name = name.to_case(Case::Snake);
  }
}

#[derive(Debug, Clone)]
enum InitState {
  Unused,
  Used { is_option: bool, needs_default: bool },
}

/// Returns `(initialized, option)`. The first bool will be false when the
/// field needs a default value.
fn init_state(instr: &[Instr], field: &str) -> InitState {
  let mut used = false;
  let mut is_option = false;
  let mut needs_default = false;
  for i in instr {
    match i {
      Instr::Set(f, val) => {
        if field == f {
          is_option = val.initial == Value::Null;
          used = true;
          needs_default = false;
          break;
        }
      }
      Instr::If(_, when_true, when_false) => {
        let mut assigned_true = false;
        let mut assigned_false = false;
        for i in when_true {
          if let Instr::Set(f, val) = i {
            if field == f {
              if val.initial == Value::Null {
                is_option = true;
              }
              assigned_true = true;
              break;
            }
          }
        }
        for i in when_false {
          if let Instr::Set(f, val) = i {
            if field == f {
              if val.initial == Value::Null {
                is_option = true;
              }
              assigned_false = true;
              break;
            }
          }
        }
        if !assigned_true && !assigned_false {
          continue;
        }
        used = true;
        match (assigned_true, assigned_false) {
          (true, true) => {
            needs_default = false;
          }
          (false, true) | (true, false) => {
            needs_default = true;
            is_option = true;
            break;
          }
          (false, false) => {}
        }
      }
      _ => {}
    }
  }
  if !used {
    InitState::Unused
  } else {
    InitState::Used { is_option, needs_default }
  }
}
