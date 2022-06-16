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
