use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Ident, ItemImpl, Lit, Meta, NestedMeta};

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
        if method.sig.ident == "new" {
          python_funcs.push(quote!(
            #[new]
            fn #py_name() {
              // Self::#name(#arg_names)
            }
          ));
          /*
          let new_attr = quote!(#[cfg_attr(feature = "python_plugins", new)]).into();
          method.attrs.push(Attribute::parse_outer.parse(new_attr).unwrap().pop().unwrap());
          */
        }
        if method.sig.receiver().is_none() {
          /*
          let new_attr = quote!(#[cfg_attr(feature = "python_plugins", staticmethod)]).into();
          method.attrs.push(Attribute::parse_outer.parse(new_attr).unwrap().pop().unwrap());
          */
          // let new_attr = quote!(#[staticmethod]).into();
          python_funcs.push(quote!(
            #[staticmethod]
            fn #py_name() {
              // Self::#name(#arg_names)
            }
          ));
        } else {
          python_funcs.push(quote!(
            fn #py_name() {
              // Self::#name(#arg_names)
            }
          ));
        }
      }
      _ => abort!(it, "only expecting methods"),
    }
  }
  let out = quote!(
    #[cfg_attr(feature = "panda_plugins", ::panda::define_ty(path = #panda_path, map_key = #panda_map_key))]
    #block

    #[cfg(feature = "python_plugins")]
    #[::pyo3::pymethods]
    impl #ty {
      #( #python_funcs )*
    }
  );
  out.into()
}
