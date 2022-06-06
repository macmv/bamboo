# Upgrading versions

I'm writing this down as notes for myself, on how to upgrade. As of writing,
I am upgrading from 1.18.1 to 1.18.2. This document should apply to all version
changes, although major version bumps will require more intervention. This should
be kept up to date, so that any maintainer can upgrade bamboo to a new Minecraft
version with relative ease.

### TDLR:
- Update `bamboo-data`, and push new json files to the website.
- Update `bb_data/src/main.rs` to have a new entry in `VERSIONS`.
- Update `bb_common/src/version.rs` to have a new entry in `ProcolVersion` and `BlockVersion`.
- Update the `latest` functions for `ProcolVersion` and `BlockVersion`.
- Recompile, see whats broken.

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
