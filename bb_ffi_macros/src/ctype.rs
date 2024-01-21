use super::gen_docs;
use proc_macro::TokenStream;
use proc_macro2::{Span};
use quote::quote;
use syn::{
  parse_macro_input, punctuated::Punctuated, AttrStyle, Attribute, Field, Fields,
  GenericArgument, Ident, ItemStruct, Path, Token, Type,
};

#[allow(clippy::collapsible_match)]
pub fn ctype(_args: TokenStream, input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as ItemStruct);

  let original_docs = gen_docs(&input);
  let mut changed = false;

  match &mut input.fields {
    Fields::Named(fields) => {
      let mut new_fields = Punctuated::<Field, Token![,]>::new();
      for mut field in fields.named.clone() {
        if let Some(host_type) = host_type(&field.ty) {
          let mut not_host = field.clone();
          field.ty = host_type;
          field.attrs.push(Attribute {
            pound_token:   Token![#](Span::call_site()),
            style:         AttrStyle::Outer,
            bracket_token: Default::default(),
            meta: syn::Meta::Path(path(&["cfg"])),
          });
          not_host.attrs.push(Attribute {
            pound_token:   Token![#](Span::call_site()),
            style:         AttrStyle::Outer,
            bracket_token: Default::default(),
            meta: syn::Meta::Path(path(&["cfg"])),
          });
          new_fields.push(field);
          new_fields.push(not_host);
          changed = true;
        } else {
          new_fields.push(field);
        }
      }
      fields.named = new_fields;
    }
    _ => {}
  };

  let input_attrs = input.attrs.clone();
  input.attrs.clear();
  let ty = &input.ident;

  let docs = if changed {
    let generated_docs = gen_docs(&input);
    quote! {
      #[doc = "Original struct:"]
      #[doc = #original_docs]
      #[doc = "Generated struct:"]
      #[doc = #generated_docs]
    }
  } else {
    quote!()
  };

  quote! {
    #(#input_attrs)*
    #docs
    #[repr(C)]
    #[derive(Clone)]
    #input

    #[cfg(feature = "host")]
    impl Copy for #ty {}
    #[cfg(feature = "host")]
    unsafe impl wasmer::ValueType for #ty {}
  }
  .into()
}

fn path(text: &[&str]) -> Path {
  let mut segments = Punctuated::new();
  for segment in text {
    segments.push(syn::PathSegment {
      ident:     Ident::new(segment, Span::call_site()),
      arguments: syn::PathArguments::None,
    });
  }
  Path { leading_colon: None, segments }
}
fn host_type(ty: &Type) -> Option<Type> {
  match ty {
    Type::Ptr(ty) => Some(Type::Path(syn::TypePath {
      qself: None,
      path:  {
        let mut segments = Punctuated::new();
        segments.push(syn::PathSegment {
          ident:     Ident::new("wasmer", Span::call_site()),
          arguments: syn::PathArguments::None,
        });
        segments.push(syn::PathSegment {
          ident:     Ident::new("WasmPtr", Span::call_site()),
          arguments: syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token:     Token![<](Span::call_site()),
            args:         {
              let mut args = Punctuated::new();
              args.push(GenericArgument::Type(*ty.elem.clone()));
              args
            },
            gt_token:     Token![>](Span::call_site()),
          }),
        });
        Path { leading_colon: None, segments }
      },
    })),
    _ => None,
  }
}
