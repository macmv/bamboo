use super::{convert, Cond, Expr, Field, Instr, Op, Packet, Type, Value};
use convert_case::{Case, Casing};

pub fn pass(p: &mut Packet) {
  let len = simplify_instr(&mut p.reader.block);
  if let Some(l) = len {
    p.reader.block = p.reader.block[..l].to_vec();
    p.fields.push(Field {
      name:        "unknown".into(),
      ty:          Type::Array(Box::new(Type::Byte)),
      reader_type: None,
      option:      false,
      initialized: false,
    });
  }
  for f in &mut p.fields {
    simplify_name(&mut f.name);
  }
}
pub fn finish(p: &mut Packet) {
  for f in &mut p.fields {
    let (initialized, option) = check_option(&p.reader.block, &f.name);
    if option && matches!(f.ty, Type::Array(_)) {
      f.option = false;
      f.initialized = false;
    } else {
      f.initialized = initialized;
      f.option = option;
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
        simplify_val(idx);
        simplify_expr(val);
      }
      Instr::Let(_, val) => {
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
      Instr::For(_, range, block) => {
        simplify_expr(&mut range.min);
        simplify_expr(&mut range.max);
        simplify_instr(block);
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
        simplify_val(len);
      }
      Instr::Return(v) => simplify_expr(v),
    }
  }
  None
}
fn set_unknown() -> Instr {
  Instr::Set(
    "unknown".into(),
    Expr::new(Value::Var(1)).op(Op::Call("tcp::Packet".into(), "read_all".into(), vec![])),
  )
}
fn simplify_cond(cond: &mut Cond) {
  simplify_cond_overwrite(cond);
}
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
  }
}
fn simplify_expr(expr: &mut Expr) {
  simplify_expr_overwrite(expr);
}
fn simplify_expr_overwrite(expr: &mut Expr) -> (bool, Option<Instr>) {
  fn simplify(expr: &mut Expr) -> (bool, Option<Instr>) {
    simplify_val(&mut expr.initial);
    for op in expr.ops.iter_mut() {
      if simplify_op(op) {
        return (true, None);
      }
    }
    (false, convert::overwrite(expr))
  }
  let res = match expr.initial {
    Value::Static(..)
    | Value::CallStatic(..)
    | Value::New(..)
    | Value::Array(_)
    | Value::Field(..) => return (true, None),
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
              "read_item" | "read_block_hit" | "read_nbt" => return (true, None),
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
fn simplify_val(val: &mut Value) {
  match val {
    Value::Field(name) => simplify_name(name),
    Value::Static(..) => {}
    Value::Array(len) => simplify_expr(len),
    Value::CallStatic(class, name, args) => {
      simplify_name(name);
      *class = convert::class(class).to_string();
      let (new_class, new_name) = convert::static_call(&class, &name);
      *class = new_class.into();
      *name = new_name.into();
      args.iter_mut().for_each(|a| simplify_expr(a))
    }
    Value::MethodRef(class, name) => {
      simplify_name(name);
      *class = convert::class(class).to_string();
      *val = convert::static_ref(&class, &name);
    }
    Value::Closure(args, block) => {
      for a in args.iter_mut() {
        simplify_expr(a);
      }
      simplify_instr(&mut block.block);
    }
    Value::New(_, args) => {
      args.iter_mut().for_each(|a| simplify_expr(a));
    }
    Value::Null | Value::Lit(_) | Value::Var(_) => {}
  }
}
fn simplify_op(op: &mut Op) -> bool {
  match op {
    Op::BitAnd(rhs) => simplify_expr(rhs),
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
        (true, _) => return true,
      }
      match simplify_expr_overwrite(val) {
        (false, _) => {}
        (true, _) => return true,
      }
    }
    Op::Call(class, name, args) => {
      *class = convert::class(class).to_string();
      simplify_name(name);
      let (new_name, new_args) = convert::member_call(class, name);
      *name = new_name.into();
      if let Some(a) = new_args {
        *args = a;
      } else {
        args.iter_mut().for_each(|a| simplify_expr(a))
      }
    }
    Op::Cast(_) => {}

    Op::As(..) | Op::Neq(..) => unreachable!(),
  }
  false
}
fn simplify_name(name: &mut String) {
  if name == "type" {
    *name = "ty".into();
  } else {
    *name = name.to_case(Case::Snake);
  }
}

/// Returns `(initialized, option)`. The first bool will be false when the
/// field needs a default value.
fn check_option(instr: &[Instr], field: &str) -> (bool, bool) {
  for i in instr {
    match i {
      Instr::Set(f, val) => {
        if field == f {
          if val.initial == Value::Null {
            return (true, true);
          }
          return (true, false);
        }
      }
      Instr::If(_, when_true, when_false) => {
        let mut assigned_true = false;
        let mut assigned_false = false;
        let mut needs_option = false;
        for i in when_true {
          match i {
            Instr::Set(f, val) => {
              if field == f {
                if val.initial == Value::Null {
                  needs_option = true;
                }
                assigned_true = true;
                break;
              }
            }
            _ => {}
          }
        }
        for i in when_false {
          match i {
            Instr::Set(f, val) => {
              if field == f {
                if val.initial == Value::Null {
                  needs_option = true;
                }
                assigned_false = true;
                break;
              }
            }
            _ => {}
          }
        }
        match (assigned_true, assigned_false) {
          (true, true) => return (true, needs_option),
          (false, true) => return (false, true),
          (true, false) => return (false, true),
          (false, false) => continue,
        }
      }
      _ => {}
    }
  }
  (false, true)
}
