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
/// # trait Behavior {}
/// # mod impls {
/// #   pub struct Chest;
/// #   pub struct Falling;
/// #   pub struct Log;
/// # }
/// # impl Behavior for impls::Chest {}
/// # impl Behavior for impls::Falling {}
/// # impl Behavior for impls::Log {}
/// # struct DefaultBehavior;
/// # impl Behavior for DefaultBehavior {}
/// # enum Kind {
/// #   Chest,
/// #   Sand, RedSand, Gravel,
/// #   OakLog, BirchLog, SpruceLog, DarkOakLog, AcaciaLog, JungleLog,
/// # }
/// fn call<R>(kind: Kind, f: impl FnOnce(&dyn Behavior) -> R) -> R {
///   bb_server_macros::behavior! {
///     // `kind` and `f` are the variables passed to this function.
///     // `Kind` is the enum we are matching against.
///     kind, f -> :Kind:
///
///     // Defines a mapping. This says `Kind::Chest` should call `f(&impls::Chest)`. The
///     // right side is any expression.
///     Chest => impls::Chest;
///
///     // Defines a mapping that matches `Kind::Sand | Kind::RedSand ...` to `f(&impls::Falling)`.
///     Sand | RedSand | Gravel => impls::Falling;
///
///     // Defines a set. This is a collection of other items.
///     *wood* = Oak, Birch, Spruce, DarkOak, Acacia, Jungle;
///
///     // Defines a mapping. This will map all possible concatenations of the `wood` set, appended
///     // with `Log`. So this will match `Kind::OakLog | Kind::BirchLog | Kind::SpruceLog ...` to
///     // `f(&impls::Log)`.
///     *wood*Log => impls::Log;
///
///     // Defines the default behavior for any `Kind` that isn't matched above.
///     _ => DefaultBehavior;
///   }
/// }
/// ```
#[proc_macro]
pub fn behavior(input: TokenStream) -> TokenStream { behavior::behavior(input) }

#[proc_macro_derive(Window, attributes(filter, output, not_inv))]
pub fn window(input: TokenStream) -> TokenStream { window::window(input) }

#[proc_macro_derive(WindowEnum, attributes(name))]
pub fn window_enum(input: TokenStream) -> TokenStream { window::window_enum(input) }
