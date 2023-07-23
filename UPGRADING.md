# Updating

Updating bamboo to a newer Minecraft version is somewhat involved. The high level steps are:
- Update `bamboo-data` to parse the new version.
- Update the versions list in the `bb_data` and `bb_common` packages.
- Open up a testing work (see `examples/debug`) and make sure all blocks match up on the latest
client.

## Updating `bamboo-data`

The first step, updating `bamboo-data`, is the most difficult. `bamboo-data` contains a decompiler
for java bytecode. While parsing the actual bytecode is done by a library, interpreting that into
usable expressions is done by custom code. Those expressions are then matched against to reconstruct
the vanilla loading process.

### Bytecode decompiler

Converting from bytecode to a syntax tree is mostly reliable, but it isn't very heavily tested.
Switch statements and complex if statements can easily cause the system to break with some
unexplainable panic (or it'll silently fail and produce an incorrect AST).

In either case, fixing this type of panic is very difficult. Because of the low test coverage, and
the small amount of tooling in place, its hard to fix something without breaking something else.

### AST analyzer

After downloading the client, `bamboo-data` will analyzer specific classes it needs. For example,
to get the list of all blocks in the game, it will parse the `Blocks.class` classfile.

With the classfile parsed, the analyzer will examine the `<clinit>` (or static constructor) of the
class. It will then search for any calls to the `register` function, and compile those into a list
of all blocks that will be registered to the game.

To find out all the metadata about each block, it will then examine the constructor of each block,
and figure out what all the block states are. This is a very fragile process, as the analyzer will
panic if it finds any function calls it does not understand.

The reasons for panicking are simple: silent failures are much more painful to debug than loud ones.

If you run into a panic while running the analyzer (which is very likely), then implement parsing
for the new functions that were called in the constructor.

### Testing `bamboo-data`

Once `bamboo-data` is able to decompile the latest version locally, you can test it with `bamboo` by
pointing the `bb_data` package at your local `bamboo-data` instance.

The file `data-config.toml` can be placed in the root of this project. When present, `bb_data` will
use it to find a local (or remote) instance of the `bamboo-data` repository. See
`data-config-example.toml` for an example of what this file should contain.

Note that this new bamboo-data version won't load until you update this repository as well.

## Updating `bamboo`

This is the easy(er) part: you just need to write a version bump into a few places. Namely:
- Update `bb_data/src/lib.rs` to have a new entry in `VERSIONS`.
  - Each one of these is a bamboo data version that will be loaded.
  - For minor versions, replace the old minor version with the new one.
  - For major versions, add a new entry to the list.
- Update `bb_common/src/version.rs` to have a new entry in `ProcolVersion`.
  - For major versions, a new `BlockVersion` will be needed as well.
  - Update the `latest` functions for `ProcolVersion` and `BlockVersion`.

Once all of this has been updated, and you have a `data-config.toml` in place, you should be able
to compile the server/proxy.

The server usually doesn't run into (that many) issues, as block data is quite uniform. The proxy,
on the other hand, needs to parse the vanilla packet readers, and compile them to rust source code.
This is somewhat fragile, and it'll break when a new version adds a new packet buffer reader
function.

## Common places things break

These are all problems I've encountered before, and how I've solved them. Feel free to add to the
list if you find any more :).

### Compile errors

These will show up as a compile error after updating `bb_data`.

#### Entity metadata

After updating `bamboo-data`, a list of all entity metadata fields will need to be synced up with
the hardcoded list in `bb_common/src/metadata/mod.rs`.

#### Packet reader functions

The vanilla client has a bunch of packet reader functions (like `read_int` for example). These all
need to be mapped by hand in `bamboo-data`, and implemented by hand in `bb_proxy`. This isn't all
that bad (theres only like 20 functions), but it does cause problems when updating. The fix is to
just implement the new functionality.


### Client errors

These will show up as an error on the client when connecting to a running instance of bamboo.

#### Dimension codecs

Dimension codecs define sky color, grass color, the height of the world, and some other important
information about each dimension. However, the client will complain if you are missing any fields,
or have any additional fields. This means that adding fields to a dimension codec will break all
previous versions.

For now, I just hardcode the dimension codec in `bb_proxy/src/packet/cb.rs`. This needs to be
updated to what the new client is expecting. After updating this, no older clients will be able to
join.

### Silent errors

This will only show up through manual testing in the debug world.

#### Mismatched blocks

Generally, this comes from `bamboo-data` missing a property when parsing the block constructor. It
will usually panic if it doesn't understand something, but sometimes it silently fails, which causes
failures like this to show up all the way at the end of the process.

Simply find the missing block, and compare the decompiled code with the `bamboo-data` JSON to figure
out what went wrong.

This happened last time with vines: they use a for loop to add properties, and the decompiler simply
ignored the for loop, and interpreted it as a single property add. The fix here was to hardcode vine
properties (parsing a for loop is quite difficult).
