use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{
  parse::Parser, parse_macro_input, Attribute, AttributeArgs, ItemImpl, Lit, Meta, NestedMeta,
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
  let mut block = parse_macro_input!(input as ItemImpl);
  for it in &mut block.items {
    match it {
      syn::ImplItem::Method(method) => {
        if method.sig.ident == "new" {
          let new_attr = quote!(#[cfg_attr(feature = "python_plugins", ::pyo3::new)]).into();
          method.attrs.push(Attribute::parse_outer.parse(new_attr).unwrap().pop().unwrap());
        }
      }
      _ => abort!(it, "only expecting methods"),
    }
  }
  quote!(
    #[cfg_attr(feature = "panda_plugins", ::panda::define_ty(path = #panda_path, map_key = #panda_map_key))]
    #[cfg_attr(feature = "python_plugins", ::pyo3::pymethods)]
    #block
  )
  .into()
}
