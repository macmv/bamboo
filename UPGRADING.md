# Upgrading versions

I'm writing this down as notes for myself, on how to upgrade. As of writing,
I am upgrading from 1.18.1 to 1.18.2. This document should apply to all version
changes, although major version bumps will require more intervention. This should
be kept up to date, so that any maintainer can upgrade bamboo to a new Minecraft
version with relative ease.

### TLDR:
- Update `bamboo-data`, and push new json files to the website.
- Update `bb_data/src/main.rs` to have a new entry in `VERSIONS`.
- Update `bb_common/src/version.rs` to have a new entry in `ProcolVersion` and `BlockVersion`.
- Update the `latest` functions for `ProcolVersion` and `BlockVersion`.
- Recompile, see what's broken.

### Details

The first thing the proxy will do is spit out an invalid version error when a client
connects with a new version. This should be fixed to just show the client an error
on the status screen.

Firstly, the `bamboo-data` repository must be updated. See the `UPGRADING.md` file in
that repository. This should be updated, then pushed to `main`, then wait for the CI
to finish. Once that is done, continue here.

The next thing to do is update `bb_data`. This has a list of every version, and how to
pull it from `bamboo-data`. Because `bb_data` generates our protocol parser, this needs
to be updated first. Simply edit the `VERSIONS` static table in `bb_data/src/lib.rs` to
add the new version. In my case, because it was a minor version bump, I just replaced the
most recent version with the new minor version.

Once `bb_data` has been updated, rebuild the server. It should fail to compile, as the
`bb_common::version::ProcolVersion` enum is out of date.

To fix this enum, edit the file `bb_common/src/version.rs`. I added 1.18.2 to the protocol
versions list, and replaced the block version 1.18.1 with 1.18.2. For a major version
bump, `BlockVersion` will need a new entry. There is also a match statement in
`ProcolVersion::block` that needs to be updated.

The last change in `version.rs` is the `latest` functions. There is one for `ProcolVersion`
and one for `BlockVersion`. These functions should both be updated to latest.

If this is a minor version bump, just recompile and see what happens! In my case, the
dimension codec was missing an entry, so I had to go edit the hardcoded dimension codec
in `bb_proxy/src/packet/cb.rs`, and then everything worked!

At the time of writing, 1.19 is not out yet (just in snapshots), so I'm not going to try
to do a major version bump. However, once I do upgrade to 1.19, I will update this file to
include all the steps that need to be completed.

EDIT:

I'm updating to 1.19 now. I'm already updated `bamboo-data`. The next step is to update
`bb_data::VERSIONS`, to have it pull the latest versions. This immediately fails, as there
is an extra enum variant added to the entity metadata. After fixing this, `bb_data` compiles,
and I updated the version in `bb_common`.

After fixing this, the server needed to be updated in two places: the `entity::MetadataType`
enum needs to match the one in `bb_data`, and the `entity::Type::is_living` function needed
to be updated for the new entities. After this, the server compiled.

The proxy got an invalid packet reader function, called `read_registry_value`. I fixed this,
along with a few other `read_` functions.

The next thing was to fix the protocol reader. A bunch of things in `bb_proxy` need manual
intervention, simply because of the nature of things like join game packets and chunks.

Long story short, just connect with a vanilla client, and fix all the bugs.

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
