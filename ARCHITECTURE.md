# Overview

Bamboo uses a proxy-server model. A proxy will accept connections from Java clients, and convert
their packets into a version-agnostic representation, which is then sent to the server. The server
stores the state of the world using the latest version of the game. It will then reply with
version-agnostic messages, which the proxy translates into version-specific packets.

The server is defined in `bb_server`, and the proxy is defined in `bb_proxy`.

The "version-agnostic" message is a `bb_common` packet, and is defined in `bb_common/src/net/cb.rs`
and `bb_common/src/net/sb.rs` (for clientbound and serverbound packets respectively).

# Blocks/items

The server must know all the blocks in the latest version of the game. The proxy must know how to
convert a block from any version of the game to the latest version of the game (and vice versa).

To achieve this, the `bamboo-data` repository downloads all versions of the game, and parses the
source code to discover all blocks from each version of the game. Then, the `bb_data` crate will
match up all blocks between versions, and create lookup tables to convert block ids to/from the
latest version.

All of the same logic applies to items.

# Packets

Packets are more complex than blocks. Instead of having uniform data stored with each block, each
packet has a set of fields, a reader function, and a writer function. These functions are
implemented in java, and must be converted to rust.

`bamboo-data` does its best to parse the packet reader, and generate a syntax tree which is then
stored in JSON. `bb_data` takes this packet reader, and then converts the syntax tree to a more
rust-specific version. It then typechecks the syntax tree, and using the result of typechecking it,
generates a writer function that does the opposite of the reader.

Once a reader and writer are generated for rust, they are compared between versions. All versions
with the same packet reader are merged into one variant of an enum, which is then written to a rust
file by the `bb_data` buildscript.

The proxy then depends on this buildscript output, and uses it to convert vanilla packets into
`bb_common` packets. This conversion is done by hand in `bb_proxy/src/packet/cb/impls.rs` and
`bb_proxy/src/packet/sb/impls.rs`.

# Crates

Below is a list of all crates in this repository, and what they do.

## `bb_macros`

Proc macro crate, which defines some small utility macros used in multiple other crates.

## `bb_server_macros`

Proc macro crate, which defines macros only used in `bb_server`.

## `bb_transfer`

The wire protocol used between the proxy and the server. This allows you to `#[derive(Transfer)]` a
struct, which is conceptually similar to `#[derive(Serialize, Deserialize)]`.

This is a custom format because it allows for the flexibility of a protobuf when adding new fields,
and the ease of using a derive macro.

## `bb_data`

This is a source code generator, which takes the data from `bamboo-data` and generates structs/enums
to use those blocks/items/packets.

## `bb_common`

This contains common utilities for `bb_proxy` and `bb_server`. This includes things like block
positions, UUIDs, all the packets sent between the server and proxy, and a low-level chunk
representation.

## `bb_server`

The Bamboo server. It listens for connections from a proxy, and handles persistent state like active
players and the current world.

## `bb_proxy`

The proxy. This is required for any clients to connection to the server. It converts the bb_transfer
protocol into the Minecraft protocol using generated code from `bb_data`.

## `bb_cli`

A cli tool, used to connect to a Minecraft server and validate that it is sending good data (things
like making sure the client won't leak chunks, checks for keep alive packets, etc).

This is not part of the server! It is only used for testing.

## `bb_ffi_macros`
  
Generates C compatible structs, and creates C compatible enums using a union and a struct.

## `bb_ffi`

This defines a bunch of ``#[repr(C)]`` structs/functions, which are used in the Wasm api. Depended
on by the server and `bb_plugin`.

## `bb_plugin`

This is a library that should be included on the plugin side. It wraps all the types in `bb_ffi`
with safe, easy to use types. To write a plugin in another language, one would need to work from
`bb_ffi`, not `bb_plugin`, as this is very Rust-specific.
