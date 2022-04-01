This server creates a music track that you can edit by placing blocks,
and play with `/play`. You can also stop the song with `/stop`.

This doesn't load a world from disk, so just `cd music` and `cargo run --bin bb_server`
to try it out.

Things that need doing:
- [ ] Sound data in `bamboo-data`. This is a large task, as there are
  many, many sounds. These need to be collected from all the versions,
  and a new column needs to be added [here](https://macmv.gitlab.io/sugarcane-data/)
  to finish this.
- [ ] `**` operator in Panda. This is a very simple fix, I just haven't
  gotten around to implementing it. To fix this, one should just implement
  the `exp` operator [here](https://gitlab.com/macmv/panda/-/blob/main/panda/src/runtime/tree/expr.rs#L409).
