use super::gen_docs;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{
  parse_macro_input,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Bracket},
  Attribute, Field, Fields, FieldsNamed, Ident, ItemEnum, ItemStruct, ItemUnion, Path,
  PathArguments, PathSegment, Token, Type, TypePath, VisPublic, Visibility,
};

macro_rules! punct {
  [ $($field:expr),* ] => {{
    let mut punct = Punctuated::new();
    punct.extend(vec![$($field),*]);
    punct
  }}
}
macro_rules! fields_named {
  { $($name:ident: $ty:expr,)* } => {
    FieldsNamed { brace_token: Brace { span: Span::call_site() }, named: punct![$(
      Field {
        attrs: vec![],
        vis: Visibility::Public(VisPublic { pub_token: Token![pub](Span::call_site()) }),
        ident: Some(Ident::new(stringify!($name), Span::call_site())),
        colon_token: Some(Token![:](Span::call_site())),
        ty: $ty,
      }
    ),*] }
  }
}
macro_rules! path {
  ( :: $($ident:ident)::* ) => {
    Path {
      leading_colon: Some(Token![::](Span::call_site())),
      segments: punct![$($ident),*],
    }
  };
  ( $($ident:ident)::* ) => {
    Path {
      leading_colon: None,
      segments: punct![$(
        PathSegment {
          ident: Ident::new(stringify!($ident), Span::call_site()),
          arguments: PathArguments::None,
        }
      ),*],
    }
  };
}

struct CEnum {
  vis:      Visibility,
  name:     Ident,
  variants: Vec<CVariant>,
}

struct CVariant {
  name: Ident,
  ty:   CVariantType,
}

enum CVariantType {
  Unit,
  Single(Type),
  Struct(Ident, Vec<(Ident, Type)>),
}

fn path(ident: Ident) -> Path {
  Path {
    leading_colon: None,
    segments:      {
      let mut punct = Punctuated::new();
      punct.extend([PathSegment { ident, arguments: PathArguments::None }]);
      punct
    },
  }
}

impl CVariant {
  pub fn ty(&self) -> Option<Type> {
    match &self.ty {
      CVariantType::Unit => None,
      CVariantType::Single(ty) => Some(ty.clone()),
      CVariantType::Struct(name, _) => {
        Some(Type::Path(TypePath { qself: None, path: path(name.clone()) }))
      }
    }
  }
  pub fn is_copy(&self) -> bool {
    match &self.ty {
      CVariantType::Unit => true,
      CVariantType::Single(ty) => is_copy(ty),
      CVariantType::Struct(_, fields) => fields.iter().all(|(_, ty)| is_copy(&ty)),
    }
  }
  pub fn lower_name(&self) -> String { to_lower(&self.name.to_string()) }
  pub fn field_name(&self) -> Ident {
    Ident::new(&format!("f_{}", to_lower(&self.name.to_string())), self.name.span())
  }
}

impl CEnum {
  pub fn new(input: ItemEnum) -> Result<Self, TokenStream2> {
    let variants = input
      .variants
      .into_iter()
      .map(|v| Ok(CVariant { name: v.ident, ty: match v.fields {
        Fields::Unit => CVariantType::Unit,
        Fields::Unnamed(unnamed) => {
          if unnamed.unnamed.len() == 1 {
            CVariantType::Single(unnamed.unnamed.first().unwrap().ty.clone())
          } else {
            return Err(quote_spanned!(unnamed.span() => compile_error!("tuple variants are not allowed");))
          }
        }
        Fields::Named(named) => {
          let mut name = None;
          for attr in v.attrs {
            if attr.path.get_ident().map(|i| i == "name").unwrap_or(false) {
              if name.is_some() {
                return Err(quote_spanned!(named.span() => compile_error!("cannot have two #[name] attributes");));
              }
              let t: TokenStream = attr.tokens.into();
              let mut iter = t.into_iter();
              iter.next().unwrap();
              let tree = iter.next().unwrap();
              name = Some(syn::parse::<syn::LitStr>(tree.into()).unwrap().parse().unwrap());
            }
          }
          if name.is_none() {
            return Err(quote_spanned!(named.span() => compile_error!("missing #[name] attribute");))
          }
          CVariantType::Struct(
            name.unwrap(),
            named.named.into_iter().map(|v| (v.ident.unwrap(), v.ty)).collect(),
          )
        }
      }}))
      .collect::<Result<Vec<_>, _>>()?;
    Ok(CEnum { vis: input.vis, name: input.ident, variants })
  }
  pub fn enum_name(&self) -> Ident { Ident::new(&format!("{}Enum", self.name), self.name.span()) }
  pub fn cenum_name(&self) -> Ident { self.name.clone() }
  pub fn data_name(&self) -> Ident { Ident::new(&format!("{}Data", self.name), self.name.span()) }

  pub fn union_fields(&self) -> FieldsNamed {
    let fields = self.variants.iter().flat_map(|v| {
      Some(Field {
        attrs:       vec![],
        vis:         Visibility::Public(VisPublic { pub_token: Token![pub](Span::call_site()) }),
        ident:       Some(v.field_name()),
        colon_token: Some(Token![:](Span::call_site())),
        ty:          match &v.ty {
          CVariantType::Unit => return None,
          CVariantType::Single(ty) => wrap_manually_drop(ty.clone()),
          CVariantType::Struct(ty_name, fields) => {
            let ty_path = Type::Path(TypePath { qself: None, path: path(ty_name.clone()) });
            if fields.iter().all(|(_, ty)| is_copy(&ty)) {
              ty_path
            } else {
              wrap_manually_drop(ty_path)
            }
          }
        },
      })
    });
    FieldsNamed {
      brace_token: Brace { span: Span::call_site() },
      named:       {
        let mut punct = Punctuated::new();
        punct.extend(fields);
        punct
      },
    }
  }

  pub fn additional_structs(&self) -> Vec<proc_macro2::TokenStream> {
    self
      .variants
      .iter()
      .flat_map(|v| match &v.ty {
        CVariantType::Struct(name, fields) => {
          let field_name = fields.iter().map(|(name, _)| name);
          let field_ty = fields.iter().map(|(_, ty)| ty);
          let copy_attr = if v.is_copy() {
            quote!(#[derive(Copy)])
          } else {
            quote!(#[cfg_attr(feature = "host", derive(Copy))])
          };
          Some(quote! {
            #[repr(C)]
            #[derive(Debug, Clone)]
            #copy_attr
            pub struct #name {
              #(pub #field_name: #field_ty,)*
            }

            #[cfg(feature = "host")]
            unsafe impl<T: Copy> wasmer::ValueType for #name {}
          })
        }
        _ => None,
      })
      .collect()
  }
  pub fn new_funcs(&self) -> Vec<proc_macro2::TokenStream> {
    self
      .variants
      .iter()
      .enumerate()
      .map(|(variant, v)| {
        let field = v.field_name();
        let new_name = Ident::new(&format!("new_{}", v.lower_name()), v.name.span());
        if let Some(ty) = v.ty() {
          let convert_manually_drop =
            if v.is_copy() { quote!(value) } else { quote!(::std::mem::ManuallyDrop::new(value)) };
          let data_name = self.data_name();
          quote!(
            pub fn #new_name(value: #ty) -> Self {
              Self {
                variant: #variant,
                data: #data_name { #field: #convert_manually_drop },
              }
            }
          )
        } else {
          quote!(
            pub fn #new_name() -> Self {
              Self {
                variant: #variant,
                data: unsafe { ::std::mem::MaybeUninit::uninit().assume_init() },
              }
            }
          )
        }
      })
      .collect()
  }
  pub fn as_funcs(&self) -> Vec<proc_macro2::TokenStream> {
    self
      .variants
      .iter()
      .enumerate()
      .map(|(variant, v)| {
        let field = v.field_name();
        let as_name = Ident::new(&format!("as_{}", v.lower_name()), v.name.span());
        if let Some(ty) = v.ty() {
          quote!(
            pub fn #as_name(&self) -> Option<&#ty> {
              if self.variant == #variant {
                unsafe {
                  Some(&self.data.#field)
                }
              } else {
                None
              }
            }
          )
        } else {
          quote!(
            pub fn #as_name(&self) -> Option<()> {
              if self.variant == #variant {
                Some(())
              } else {
                None
              }
            }
          )
        }
      })
      .collect()
  }
  pub fn is_funcs(&self) -> Vec<proc_macro2::TokenStream> {
    self
      .variants
      .iter()
      .enumerate()
      .map(|(variant, v)| {
        let is_name = Ident::new(&format!("is_{}", v.lower_name()), v.name.span());
        quote!(
          pub fn #is_name(&self) -> bool { self.variant == #variant }
        )
      })
      .collect()
  }
  pub fn into_funcs(&self) -> Vec<proc_macro2::TokenStream> {
    self
      .variants
      .iter()
      .enumerate()
      .map(|(variant, v)| {
        let field = v.field_name();
        let into_name = Ident::new(&format!("into_{}", v.lower_name()), v.name.span());
        if let Some(ty) = v.ty() {
          let convert_manually_drop = if v.is_copy() {
            quote!(me.data.#field)
          } else {
            quote!(::std::mem::ManuallyDrop::take(&mut me.data.#field))
          };
          quote!(
            #[allow(unused_parens)]
            pub fn #into_name(mut self) -> Option<#ty> {
              let mut me = ::std::mem::ManuallyDrop::new(self);
              if me.variant == #variant {
                unsafe {
                  Some(#convert_manually_drop)
                }
              } else {
                // Drop self if we have the wrong variant.
                ::std::mem::ManuallyDrop::into_inner(me);
                None
              }
            }
          )
        } else {
          quote!(
            pub fn #into_name(mut self) -> Option<()> {
              let mut me = ::std::mem::ManuallyDrop::new(self);
              if me.variant == #variant {
                Some(())
              } else {
                // Drop self if we have the wrong variant.
                ::std::mem::ManuallyDrop::into_inner(me);
                None
              }
            }
          )
        }
      })
      .collect()
  }
  pub fn into_cenum(&self) -> proc_macro2::TokenStream {
    let cenum_name = self.cenum_name();
    let data_name = self.data_name();
    let into_case = self
      .variants
      .iter()
      .enumerate()
      .map(|(variant, v)| {
        let name = &v.name;
        let field = v.field_name();
        match &v.ty {
          CVariantType::Unit => {
            quote!(Self::#name => #cenum_name { variant: #variant, data: unsafe { ::std::mem::MaybeUninit::uninit().assume_init() } })
          }
          CVariantType::Single(_) => {
            let convert_manually_drop =
              if v.is_copy() { quote!(value) } else { quote!(::std::mem::ManuallyDrop::new(value)) };
            quote!(Self::#name(value) => #cenum_name {
              variant: #variant,
              data: #data_name {
                #field: #convert_manually_drop
              }
            })
          }
          CVariantType::Struct(struct_name, fields) => {
            let field_name = fields.iter().map(|(name, _)| name).collect::<Vec<_>>();
            let value = quote!(#struct_name { #(#field_name,)* });
            let convert_manually_drop =
              if v.is_copy() { quote!(#value) } else { quote!(::std::mem::ManuallyDrop::new(#value)) };
            quote! {
              Self::#name { #(#field_name,)* } => #cenum_name {
                variant: #variant,
                data: #data_name {
                  #field: #convert_manually_drop,
                }
              }
            }
          }
        }
      });
    quote! {
      pub fn into_cenum(self) -> #cenum_name {
        match self {
          #(#into_case,)*
        }
      }
    };
    quote!(
      pub fn into_cenum(self) -> #cenum_name { todo!() }
    )
  }
  pub fn into_renum(&self) -> proc_macro2::TokenStream {
    let enum_name = self.enum_name();
    let into_case = self.variants.iter().enumerate().map(|(variant, v)| {
      let name = &v.name;
      let field = v.field_name();
      match &v.ty {
        CVariantType::Unit => {
          quote!(#variant => #enum_name::#name)
        }
        CVariantType::Single(_) => {
          let convert_manually_drop = if v.is_copy() {
            quote!(self.data.#field)
          } else {
            quote!(::std::mem::ManuallyDrop::take(&mut self.data.#field))
          };
          quote!(#variant => #enum_name::#name(#convert_manually_drop))
        }
        CVariantType::Struct(_, fields) => {
          let field_name = fields.iter().map(|(name, _)| name).collect::<Vec<_>>();
          quote! {
            #variant => #enum_name::#name {
              #(#field_name: self.data.#field.#field_name,)*
            }
          }
        }
      }
    });
    quote! {
      pub fn into_renum(mut self) -> #enum_name {
        unsafe {
          match self.variant {
            #(#into_case,)*
            _ => panic!("invalid variant: {}", self.variant),
          }
        }
      }
    }
  }
}

#[allow(clippy::collapsible_match)]
pub fn cenum(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ItemEnum);
  let original_docs = gen_docs(&input);

  if input.variants.is_empty() {
    let name = &input.ident;
    let input_attrs = &input.attrs;
    return quote!(
      #(#input_attrs)*
      #[doc = "Original enum:"]
      #[doc = #original_docs]
      #[repr(C)]
      #[derive(Clone)]
      pub struct #name {}
    )
    .into();
  }

  let mut input_enum = input.clone();
  for variant in &mut input_enum.variants {
    variant.attrs.retain(|attr| attr.path.get_ident().map(|i| i != "name").unwrap_or(true));
  }

  let name = input.ident.clone();
  let input = match CEnum::new(input) {
    Ok(v) => v,
    Err(e) => {
      // Adding an empty struct reduces the number of other errors here.
      return quote! {
        #e
        pub struct #name {}
      }
      .into();
    }
  };

  let fields = input.union_fields();

  input_enum.ident = input.enum_name();
  let name = input.cenum_name();
  let data_name = input.data_name();
  let clone_match_cases = input.variants.iter().enumerate().map(|(variant, v)| {
    let field = v.field_name();
    if v.ty().is_some() {
      quote!(
        #variant => #data_name { #field: self.data.#field.clone() },
      )
    } else {
      quote!(
        #variant => unsafe { ::std::mem::MaybeUninit::uninit().assume_init() },
      )
    }
  });
  let drop_match_cases = input.variants.iter().enumerate().flat_map(|(variant, v)| {
    if v.is_copy() {
      None
    } else {
      let field = v.field_name();
      Some(quote!(
        #variant => ::std::mem::ManuallyDrop::drop(&mut self.data.#field),
      ))
    }
  });
  let debug_match_cases = input.variants.iter().enumerate().map(|(variant, v)| {
    let field = v.field_name();
    let fmt_str = format!("{}({{:?}})", v.name);
    if v.ty().is_some() {
      quote!(
        #variant => write!(f, #fmt_str, self.data.#field.clone()),
      )
    } else {
      quote!(
        #variant => write!(f, #fmt_str, ()),
      )
    }
  });

  let gen_struct = ItemStruct {
    attrs:        vec![Attribute {
      pound_token:   Token![#](Span::call_site()),
      style:         syn::AttrStyle::Outer,
      bracket_token: Bracket { span: Span::call_site() },
      path:          path!(repr),
      tokens:        quote!((C)),
    }],
    vis:          input.vis.clone(),
    struct_token: Token![struct](Span::call_site()),
    ident:        name.clone(),
    generics:     syn::Generics {
      lt_token:     None,
      params:       Punctuated::new(),
      gt_token:     None,
      where_clause: None,
    },
    fields:       Fields::Named(fields_named! {
      variant: Type::Path(TypePath { qself: None, path: path!(usize) }),
      data: Type::Path(TypePath { qself: None, path: data_name.clone().into() }),
    }),
    semi_token:   None,
  };
  let gen_union = ItemUnion {
    attrs: vec![Attribute {
      pound_token:   Token![#](Span::call_site()),
      style:         syn::AttrStyle::Outer,
      bracket_token: Bracket { span: Span::call_site() },
      path:          path!(repr),
      tokens:        quote!((C)),
    }],
    vis: Visibility::Public(VisPublic { pub_token: Token![pub](Span::call_site()) }),
    union_token: Token![union](Span::call_site()),
    ident: data_name.clone(),
    generics: gen_struct.generics.clone(),
    fields,
  };

  let struct_docs = gen_docs(&gen_struct);
  let union_docs = gen_docs(&gen_union);

  let input_attrs: Vec<i32> = vec![];
  let additional_structs = input.additional_structs();
  let new_funcs = input.new_funcs();
  let as_funcs = input.as_funcs();
  let is_funcs = input.is_funcs();
  let into_funcs = input.into_funcs();
  let into_cenum = input.into_cenum();
  let into_renum = input.into_renum();

  let enum_name = input.enum_name();

  let out = quote! {
    #(#input_attrs)*
    /// This enum has been converted into a C safe struct and union
    #[doc = concat!("(see [`", stringify!(#name), "`] and [`", stringify!(#data_name), "`]).")]
    ///
    /// This struct and union are designed to have any bit configuration, and still be safe
    /// to use (on the host). This means that if the `variant` is invalid, the union will
    /// contain garbage data. In the `Clone` impl, the union is literally filled with
    /// `MaybeUninit::uninit().assume_init()`. This is safe, because all the `as_` functions
    /// will return `None` in this case.
    ///
    /// On the plugin, an invalid variant means something has gone wrong, and the plugin should
    /// drop this enum. For a plugin, it is *not* safe to access an invalid variant. However,
    /// the only way this can be produced is through the server doing something incorrectly,
    /// so this doesn't need to be validated on the plugin.
    ///
    /// In order for this to truly be valid in every bit configuration, the variant can be
    /// changed without modifying the union. This means that every type in the union must
    /// be valid in any bit configuration. I don't enforce this, but this means that every
    /// variant should implement `wasmer::ValueType`.
    ///
    #[doc = "Original enum:"]
    #[doc = #original_docs]
    #[doc = "Converted to struct:"]
    #[doc = #struct_docs]
    #[doc = "Along with the union:"]
    #[doc = #union_docs]
    #input_enum
    #[doc = concat!("See [`", stringify!(#enum_name), "`].")]
    #[allow(unused_parens)]
    #[cfg_attr(feature = "host", derive(Clone))]
    #gen_struct
    #[doc = concat!("See [`", stringify!(#enum_name), "`].")]
    #[allow(unused_parens)]
    #[cfg_attr(feature = "host", derive(Clone))]
    #gen_union

    #[cfg(feature = "host")]
    impl Copy for #name {}
    #[cfg(feature = "host")]
    impl Copy for #data_name {}
    #[cfg(feature = "host")]
    unsafe impl wasmer::ValueType for #name {}
    #[cfg(feature = "host")]
    unsafe impl wasmer::ValueType for #data_name {}

    impl #enum_name {
      /// Converts this enum into a C safe version, using a union.
      #into_cenum
    }

    impl #name {
      #(#new_funcs)*
      /// Converts this C enum back into a Rust enum, that cannot be used accross FFI.
      #into_renum
      #(#as_funcs)*
      #(#is_funcs)*
      #(#into_funcs)*
    }

    #(#additional_structs)*

    impl Clone for #name {
      fn clone(&self) -> Self {
        unsafe {
          #name {
            variant: self.variant,
            data: match self.variant {
               #(#clone_match_cases)*
              _ => ::std::mem::MaybeUninit::uninit().assume_init(),
            },
          }
        }
      }
    }
    #[cfg(not(feature = "host"))]
    impl Drop for #name {
      fn drop(&mut self) {
        unsafe {
          match self.variant {
            #(#drop_match_cases)*
            _ => {}
          }
        }
      }
    }
    impl ::std::fmt::Debug for #name {
      fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        unsafe {
          match self.variant {
            #(#debug_match_cases)*
            _ => write!(f, "<unknown variant {}>", self.variant),
          }
        }
      }
    }
  };
  out.into()
}

fn to_lower(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    if c.is_ascii_uppercase() {
      if !out.is_empty() {
        out.push('_');
      }
      out.push(c.to_ascii_lowercase());
    } else {
      out.push(c);
    }
  }
  out
}

fn is_copy(ty: &Type) -> bool {
  match ty {
    Type::Path(ty) => {
      let ident = &ty.path.segments.first().unwrap().ident;
      if ident == "COpt" {
        is_copy(match &ty.path.segments.first().unwrap().arguments {
          PathArguments::AngleBracketed(args) => match args.args.first().unwrap() {
            syn::GenericArgument::Type(ty) => ty,
            _ => unreachable!(),
          },
          _ => unreachable!(),
        })
      } else {
        ident == "u8"
          || ident == "i8"
          || ident == "u16"
          || ident == "i16"
          || ident == "u32"
          || ident == "i32"
          || ident == "u64"
          || ident == "i64"
          || ident == "f32"
          || ident == "f64"
          || ident == "CBool"
          || ident == "CPos"
      }
    }
    Type::Tuple(ty) => ty.elems.iter().all(is_copy),
    _ => todo!("type {ty:?}"),
  }
}

fn wrap_manually_drop(ty: Type) -> Type {
  if is_copy(&ty) {
    ty
  } else {
    Type::Path(TypePath {
      qself: None,
      path:  Path {
        leading_colon: Some(Token![::](Span::call_site())),
        segments:      punct![
          Ident::new("std", Span::call_site()).into(),
          Ident::new("mem", Span::call_site()).into(),
          PathSegment {
            ident:     Ident::new("ManuallyDrop", Span::call_site()),
            arguments: PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
              colon2_token: None,
              lt_token:     Token![<](Span::call_site()),
              args:         punct![syn::GenericArgument::Type(ty)],
              gt_token:     Token![>](Span::call_site()),
            }),
          }
        ],
      },
    })
  }
}
