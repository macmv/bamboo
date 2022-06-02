#![allow(clippy::single_match)]

use proc_macro::TokenStream;

mod behavior;
mod define_ty;
mod window;

#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn define_ty(args: TokenStream, input: TokenStream) -> TokenStream {
  define_ty::define_ty(args, input)
}

/// Creates a match statement for item/block behaviors. This is only for builtin
/// behaviors, and doesn't include plugins.
///
/// The syntax is as follows (comments are ignored):
///
/// ```
/// bb_plugin_macros::behavior! {
///   // Sets the base enum name for everything. `Kind::` will be appended to the mappings below.
///   :Kind:
///
///   // Defines a mapping. This says `Kind::Chest` should return a `Box::new(impls::Chest)`. The
///   // right side is any expression.
///   Chest => impls::Chest;
///
///   // Defines a mapping that matches `Kind::Sand | Kind::RedSand ...` to `Box::new(impls::Falling)`.
///   Sand | RedSand | Gravel => impls::Falling;
///
///   // Defines a set. This is a collection of other items.
///   *wood* = Oak, Birch, Spruce, DarkOak, Acacia, Jungle;
///
///   // Defins a mapping. This will map all possible concatenations of the `wood` set, appended
///   // with `Log`. So this will match `Kind::OakLog | Kind::BirchLog | Kind::SpruceLog ...` to
///   // `Box::new(impls::Log)`.
///   *wood*Log => impls::Log;
/// };
/// ```
#[proc_macro]
pub fn behavior(input: TokenStream) -> TokenStream { behavior::behavior(input) }

#[proc_macro_derive(Window, attributes(filter, output, not_inv))]
pub fn window(input: TokenStream) -> TokenStream { window::window(input) }

#[proc_macro_derive(WindowEnum, attributes(name))]
pub fn window_enum(input: TokenStream) -> TokenStream { window::window_enum(input) }
