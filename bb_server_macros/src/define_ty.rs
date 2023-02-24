use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort, emit_error};
use quote::quote;
use std::collections::HashMap;
use syn::{
  braced,
  parse::{Parse, ParseStream},
  parse_macro_input, Attribute, FnArg, Ident, ItemFn, LitBool, LitStr, Path, PathArguments, Result,
  ReturnType, Token, Type,
};

struct Impl {
  meta:  Vec<Attribute>,
  ty:    Type,
  info:  Info,
  funcs: Vec<ItemFn>,
}
#[derive(Clone, Debug)]
enum Info {
  Bool(bool),
  String(String),
  Type(Path),
  Object(HashMap<String, Info>),
}

impl Parse for Impl {
  fn parse(input: ParseStream) -> Result<Self> {
    let meta = input.call(Attribute::parse_outer)?;
    let _: Token![impl] = input.parse()?;
    let ty: Type = input.parse()?;
    let content;
    braced!(content in input);
    let mut info = None;
    let mut funcs = vec![];
    while !content.is_empty() {
      let lookahead = content.lookahead1();
      if lookahead.peek(Ident) {
        let _: Ident = content.parse()?;
        let _: Token![!] = content.parse()?;
        info = Some(content.parse()?);
      } else {
        funcs.push(content.parse()?);
      }
    }
    Ok(Impl {
      meta,
      ty,
      info: info.unwrap_or_else(|| abort!(content.span(), "need info block")),
      funcs,
    })
  }
}

impl Parse for Info {
  fn parse(input: ParseStream) -> Result<Self> {
    let lookahead = input.lookahead1();
    if lookahead.peek(Ident) {
      input.parse().map(Info::Type)
    } else if lookahead.peek(LitBool) {
      let lit: LitBool = input.parse()?;
      Ok(Info::Bool(lit.value()))
    } else if lookahead.peek(LitStr) {
      let lit: LitStr = input.parse()?;
      Ok(Info::String(lit.value()))
    } else if lookahead.peek(syn::token::Brace) {
      let mut values = HashMap::new();
      let content;
      braced!(content in input);
      while !content.is_empty() {
        let ident: Ident = content.parse()?;
        let _: Token![:] = content.parse()?;
        let value: Info = content.parse()?;
        let _: Token![,] = content.parse()?;
        values.insert(ident.to_string(), value);
      }
      Ok(Info::Object(values))
    } else {
      Err(lookahead.error())
    }
  }
}

#[allow(unused)]
impl Info {
  fn at(&self, path: &[&str]) -> &Info {
    self.get(path).unwrap_or_else(|| panic!("no such element at {path:?}"))
  }
  fn get(&self, path: &[&str]) -> Option<&Info> {
    if path.is_empty() {
      Some(self)
    } else {
      match self {
        Self::Object(v) => v.get(path[0]).and_then(|v| v.get(&path[1..])),
        _ => None,
      }
    }
  }

  fn get_object(&self) -> &HashMap<String, Info> {
    self.as_object().unwrap_or_else(|| panic!("not an object: {self:?}"))
  }
  fn as_object(&self) -> Option<&HashMap<String, Info>> {
    match self {
      Self::Object(v) => Some(v),
      _ => None,
    }
  }

  fn get_type(&self) -> &Path { self.as_type().unwrap_or_else(|| panic!("not a type: {self:?}")) }
  fn as_type(&self) -> Option<&Path> {
    match self {
      Self::Type(v) => Some(v),
      _ => None,
    }
  }

  fn get_str(&self) -> &str { self.as_str().unwrap_or_else(|| panic!("not a string: {self:?}")) }
  fn as_str(&self) -> Option<&str> {
    match self {
      Self::String(v) => Some(&v),
      _ => None,
    }
  }

  fn get_bool(&self) -> bool { self.as_bool().unwrap_or_else(|| panic!("not a bool: {self:?}")) }
  fn as_bool(&self) -> Option<bool> {
    match self {
      Self::Bool(v) => Some(*v),
      _ => None,
    }
  }
}

pub fn define_ty(_: TokenStream, input: TokenStream) -> TokenStream {
  let block = parse_macro_input!(input as Impl);
  let ty = &block.ty;
  let mut python_funcs = vec![];
  let mut panda_funcs = vec![];
  for method in &block.funcs {
    let name = &method.sig.ident;
    let name_str = name.to_string();
    let py_name = Ident::new(&format!("py_{}", method.sig.ident), name.span());
    let py_args = python_args(method.sig.inputs.iter());
    let py_arg_names = python_arg_names(method.sig.inputs.iter());
    let (py_ret, conv_ret) = python_ret(&method.sig.output);
    panda_funcs.push(method);
    if name == "new" {
      python_funcs.push(quote!(
        #[new]
        fn #py_name(#(#py_args),*) #py_ret {
          Self::#name(#(#py_arg_names),*) #conv_ret
        }
      ));
    } else if method.sig.receiver().is_none() {
      python_funcs.push(quote!(
        #[staticmethod]
        #[pyo3(name = #name_str)]
        fn #py_name(#(#py_args),*) #py_ret {
          Self::#name(#(#py_arg_names),*) #conv_ret
        }
      ));
    } else {
      python_funcs.push(quote!(
        #[pyo3(name = #name_str)]
        fn #py_name(#(#py_args),*) #py_ret {
          Self::#name(#(#py_arg_names),*) #conv_ret
        }
      ));
    }
  }
  let meta = block.meta;
  let panda_path = block.info.at(&["panda", "path"]).get_str();
  let panda_map_key = block.info.get(&["panda", "map_key"]).map(|v| v.get_bool()).unwrap_or(false);
  let mut derives = vec![];
  if block.info.get(&["clone"]).map(|v| v.get_bool()).unwrap_or(true) {
    derives.push("Clone");
  }
  if block.info.get(&["debug"]).map(|v| v.get_bool()).unwrap_or(true) {
    derives.push("Debug");
  }
  if block.info.get(&["eq"]).map(|v| v.get_bool()).unwrap_or(false) {
    derives.push("Hash");
    derives.push("PartialEq");
    derives.push("Eq");
  }
  let derives = derives.into_iter().map(|i| Ident::new(i, Span::call_site()));
  let derives = quote!(#[derive(#( #derives ),*)]);
  let struct_def = if block.info.get(&["struct_def"]).map(|v| v.get_bool()).unwrap_or(true) {
    let mut fields =
      block.info.get(&["fields"]).map(|f| f.get_object().clone()).unwrap_or_default();
    let mut class_name = block.info.get(&["python", "class"]).map(|v| v.get_str().to_string());
    let from_impl = match block.info.get(&["wrap"]) {
      Some(wrapped) => {
        fields.insert("inner".into(), wrapped.clone());
        let wrapped = wrapped.get_type();
        if class_name.is_none() {
          class_name = wrapped.get_ident().map(|v| v.to_string());
        }
        Some(quote!(
          impl From<#wrapped> for #ty {
            fn from(v: #wrapped) -> Self {
              Self { inner: v }
            }
          }

          impl crate::plugin::IntoPanda for #wrapped {
            type Panda = #ty;
            fn into_panda(self) -> #ty { self.into() }
          }
        ))
      }
      None => None,
    };
    let fields = fields.into_iter().map(|(k, v)| {
      let k = Ident::new(&k, Span::call_site());
      let v = v.get_type();
      quote!(pub #k: #v,)
    });
    let class_name = match class_name {
      Some(name) => quote!(name = #name),
      None => {
        emit_error!(block.ty, "must set python class name");
        quote!()
      }
    };
    Some(quote!(
      #derives
      #[cfg_attr(feature = "python_plugins", ::pyo3::pyclass(#class_name))]
      pub struct #ty {
        #( #fields )*
      }

      #from_impl
    ))
  } else {
    None
  };
  let out = quote!(
    #struct_def

    #( #meta )*
    #[cfg(feature = "panda_plugins")]
    #[::panda::define_ty(path = #panda_path, map_key = #panda_map_key)]
    impl #ty {
      #( #panda_funcs )*
    }

    #[cfg(feature = "python_plugins")]
    #[::pyo3::pymethods]
    impl #ty {
      #( #python_funcs )*
    }
  );
  // Will print the result of this proc macro
  /*
  let mut p =
    std::process::Command::new("rustfmt").stdin(std::process::Stdio::piped()).spawn().unwrap();
  std::io::Write::write_all(p.stdin.as_mut().unwrap(), out.to_string().as_bytes()).unwrap();
  p.wait_with_output().unwrap();
  */

  out.into()
}

fn python_args<'a>(args: impl Iterator<Item = &'a FnArg>) -> Vec<impl quote::ToTokens> {
  args
    .map(|a| match a {
      FnArg::Receiver(_) => quote!(#a),
      FnArg::Typed(ty) => {
        let name = &ty.pat;
        match &*ty.ty {
          Type::Path(path) => match path.path.segments[0].ident.to_string().as_str() {
            "bool" | "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128"
            | "i128" | "f32" | "f64" => {
              quote!(#name: #path)
            }
            // Assume this is a Box<dyn Callback>
            "Box" => quote!(#name: ::pyo3::PyObject),
            "Var" => quote!(#name: i32),
            "Callback" => quote!(#name: ::pyo3::PyObject),
            "Closure" => quote!(#name: ::pyo3::PyObject),
            "Vec" => quote!(#name: Vec<::pyo3::PyObject>),
            _ => quote!(#name: #path),
          },
          Type::Reference(path) => match &*path.elem {
            Type::Path(path) => match path.path.segments[0].ident.to_string().as_str() {
              "str" => quote!(#name: String),
              _ => quote!(#name: #path),
            },
            _ => quote!(#name: #path),
          },
          _ => quote!(#name: #ty),
        }
      }
    })
    .collect()
}
fn python_arg_names<'a>(args: impl Iterator<Item = &'a FnArg>) -> Vec<impl quote::ToTokens> {
  args
    .map(|a| match a {
      FnArg::Receiver(_) => quote!(self),
      FnArg::Typed(ty) => {
        let name = &ty.pat;
        match &*ty.ty {
          Type::Path(path) => match path.path.segments[0].ident.to_string().as_str() {
            "bool" | "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128"
            | "i128" | "f32" | "f64" => {
              quote!(#name)
            }
            "Box" => quote!(Box::new(#name)),
            "Var" => quote!(#name.into()),
            "Callback" => quote!(todo!("callbacks in python")),
            "Closure" => quote!(todo!("closures in python")),
            "Vec" => quote!(todo!("vecs in python")),
            // _ => abort!(ty.ty, "cannot handle type"),
            _ => quote!(#name),
          },
          Type::Reference(path) => match &*path.elem {
            Type::Path(path) => match path.path.segments[0].ident.to_string().as_str() {
              "str" => quote!(#name.as_str()),
              _ => quote!(&#name),
            },
            _ => abort!(ty.ty, "cannot handle type"),
          },
          _ => abort!(ty.ty, "cannot handle type"),
        }
      }
    })
    .collect()
}

fn python_ret(out: &ReturnType) -> (impl quote::ToTokens, Option<impl quote::ToTokens>) {
  (
    match &out {
      ReturnType::Type(_, ty) => match &**ty {
        Type::Path(path) => match path.path.segments[0].ident.to_string().as_str() {
          "Result" => {
            let arg = match &path.path.segments[0].arguments {
              PathArguments::AngleBracketed(args) => args.args.first().cloned().unwrap(),
              _ => unreachable!(),
            };
            return (
              quote!(-> ::pyo3::PyResult<#arg>),
              Some(quote!(.map_err(crate::plugin::python::conv_err))),
            );
          }
          "Var" => {
            return (quote!(-> ::pyo3::PyObject), Some(quote!(; todo!("convert var to python"))))
          }
          "Vec" => {
            return (quote!(-> ::pyo3::PyObject), Some(quote!(; todo!("convert vec to python"))))
          }
          _ => quote!(#out),
        },
        _ => quote!(#out),
      },
      _ => quote!(),
    },
    None,
  )
}
