use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{
  parse_macro_input, AttributeArgs, FnArg, Ident, ItemImpl, Lit, Meta, NestedMeta, PathArguments,
  ReturnType, Type,
};

pub fn define_ty(args: TokenStream, input: TokenStream) -> TokenStream {
  let args = parse_macro_input!(args as AttributeArgs);
  let mut panda_path = None;
  let mut panda_map_key = false;
  for v in args {
    match v {
      NestedMeta::Meta(m) => match m {
        Meta::NameValue(v) => match &v.path.segments[0].ident {
          n if n == "panda_path" => match v.lit {
            Lit::Str(l) => panda_path = Some(l.value()),
            l => abort!(l, "expected str"),
          },
          n if n == "panda_map_key" => match v.lit {
            Lit::Bool(l) => panda_map_key = l.value(),
            l => abort!(l, "expected bool"),
          },
          name => abort!(name, "unknown arg {}", name),
        },
        m => abort!(m, "unknown arg {:?}", m),
      },
      _ => abort!(v, "unknown arg {:?}", v),
    }
  }
  let block = parse_macro_input!(input as ItemImpl);
  let ty = &block.self_ty;
  let mut python_funcs = vec![];
  for it in &block.items {
    match it {
      syn::ImplItem::Method(method) => {
        let name = &method.sig.ident;
        let py_name = Ident::new(&format!("py_{}", method.sig.ident), name.span());
        let py_args = python_args(method.sig.inputs.iter());
        let py_arg_names = python_arg_names(method.sig.inputs.iter());
        let (py_ret, conv_ret) = python_ret(&method.sig.output);
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
            fn #py_name(#(#py_args),*) {
              // Self::#name(#arg_names)
            }
          ));
        } else {
          python_funcs.push(quote!(
            fn #py_name(#(#py_args),*) {
              // Self::#name(#arg_names)
            }
          ));
        }
      }
      _ => abort!(it, "only expecting methods"),
    }
  }
  let out = quote!(
    #[cfg(feature = "panda_plugins")]
    #[::panda::define_ty(path = #panda_path, map_key = #panda_map_key)]
    #block

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
            "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128" | "i128"
            | "f32" | "f64" | "Vec" => {
              quote!(#name: #path)
            }
            // Assume this is a Box<dyn Callback>
            "Box" => quote!(#name: ::pyo3::PyObject),
            "Var" => quote!(#name: i32),
            "Callback" => quote!(#name: Callback),
            _ => abort!(ty.ty, "cannot handle type"),
          },
          Type::Reference(path) => match &*path.elem {
            Type::Path(path) => match path.path.segments[0].ident.to_string().as_str() {
              "str" => quote!(#name: String),
              _ => quote!(#name: #path),
            },
            _ => abort!(ty.ty, "cannot handle type"),
          },
          _ => abort!(ty.ty, "cannot handle type"),
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
            "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128" | "i128"
            | "f32" | "f64" | "Vec" => {
              quote!(#name)
            }
            "Box" => quote!(Box::new(#name)),
            "Var" => quote!(#name),
            "Callback" => quote!(#name),
            _ => abort!(ty.ty, "cannot handle type"),
          },
          Type::Reference(path) => match &*path.elem {
            Type::Path(path) => match path.path.segments[0].ident.to_string().as_str() {
              "str" => quote!(#name.as_str()),
              _ => quote!(#name),
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
          "Var" => quote!(-> ::pyo3::PyObject),
          _ => quote!(#out),
        },
        _ => abort!(out, "cannot handle ret"),
      },
      _ => quote!(),
    },
    None,
  )
}
