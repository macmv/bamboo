<div align="center">
  <h1>
    <img src="https://gitlab.com/macmv/bamboo/-/raw/main/icon.png" width=35>
    Bamboo
    <img src="https://gitlab.com/macmv/bamboo/-/raw/main/icon.png" width=35>
  </h1>

  [![pipeline status](https://gitlab.com/macmv/bamboo/badges/main/pipeline.svg)](https://gitlab.com/macmv/bamboo/-/pipelines)
  [![coverage report](https://gitlab.com/macmv/bamboo/badges/main/coverage.svg)](https://app.codecov.io/gl/macmv/bamboo)
  [![Discord](https://badgen.net/badge/icon/discord?icon=discord&label)](https://discord.gg/8CTr3N9yzU)

  A safe, fast, and secure Minecraft server optimized for minigames.
</div>

### How to run

Install [rust](https://www.rust-lang.org/learn/get-started), and then clone
this repository:

```
git clone https://gitlab.com/macmv/bamboo.git
cd bamboo
```

Now you need to run the server and proxy. Run the server with this command:

```
cargo run --bin bb_server --release
```

And run the proxy with this command:

```
cargo run --bin bb_proxy --release
```

You need the server and proxy to be running at the same time in order to connect.
The proxy will listen for Minecraft clients on port `25565`, and then it will
connect to the server on port `8483`. If you get errors when starting the server,
make sure nothing else is using that port.

The `--release` flag will make the server/proxy faster at runtime, but take
longer to compile. I recommend using the flag for both, unless you are developing
the server or proxy.

After running the server or proxy, a file named `server.toml` and `proxy.toml`
will be created. These files will have the default configuration for the server
and proxy. Here you can do things like change the world generation, enable/disable
plugins, and change online mode for the proxy.

Feel free to ask questions in the [discord](https://discord.gg/8CTr3N9yzU)!

### Writing a Minigame

Panda is the language used for plugins in this server. See the
[examples](https://gitlab.com/macmv/bamboo/-/tree/main/examples) directory
for some examples.

The [docs](https://macmv.gitlab.io/bamboo/doc/panda/index.html) are
kept up to date by pipelines, and those should be helpful when writing plugins.

This language is mostly complete, but the actual interface with the server from
Panda is not complete at all. These plugins are mostly there to test the
API that I'm writing, and they should contain examples of the newest features
as I develop them.

### Features

All of these are planned. I have no time frame for doing these things, but here
is what I would like to see in a 1.0 release:

- [ ] Minigame lobbies
  - [ ] The 'default chunk' concept
    - In a minigame lobby, most chunks are the same. The void chunks that are in
      render distance, but all empty are still stored in a vanilla server. A
      default chunk fixes this, and uses a single chunk to send data about all
      of the empty chunks in the world. This can also be used to optimize things
      like superflat worlds. This is also why the `Chunk` type does not know
      it's own position.
  - [ ] Unbreakable world
    - This needs to be a flag, for both placing and breaking blocks. It should
      be very easy to toggle with an admin command.
  - [ ] Ability to switch worlds
    - This is a server-only task. It involves clearing all the loaded chunks of
      the client, sending them a dimension change packet, loading all the new chunks,
      and spawning them into the new world. This will not be very difficult.
  - [ ] Ability to switch servers
    - This is a proxy-based task. It involves a new custom packet that the
      server sends to the 'client', that the proxy will intercept. This is a
      more difficult task than switching worlds, but is more valuable for
      scalability.
- [ ] Some form of terrain generation
  - Default chunks are good, and fast, but having terrain generation is very
    fun. I don't want this to be a minigame-only server, and I really like
    watching my own terrain generate at very fast speeds.
  - [ ] Default chunk/Terrain generation should be easily toggleable with an
        admin command.
- [ ] (Maybe) support 1000 players on one server
  - Everything is very multithreaded. This is a very general goal, and is put
    here as a reminder that things like tick loops are separate for each player.
- [x] Plugin loading in Panda (custom language)
  - I really tried to use another language for plugins. It would have been so
    much simpler to just deal with Python or JavaScript, but I couldn't stand
    the awful API. So I wrote an entire language for plugins. It's called Panda,
    and it's specific to this server. You can check it out
    [here](https://gitlab.com/macmv/panda).
- [ ] (Unlikely) Plugin loading in Rust
  - This is a dynamic linking problem. It shouldn't be very difficult, and will
    allow pluggable functionality from multiple plugins, all without
    re-compiling the server. This would act very similar to something like
    Spigot.
  - This is not a feature I am very interested in. Panda is turning out to
    be very powerful, and I would like to focus on that. If anyone wants to implement
    this, feel free to do so. Just note that it must be compatible with Panda
    plugins. You need to be able to load both Rust and Panda plugins at the
    same time.
- [ ] Plugin loading via sockets
  - Sending messages over a unix socket is fast, and would work pretty well for
    loading a plugin. At the time of writing, there is a simple python plugin,
    which can send chat messages and get blocks over a socket. This interface is
    far more annoying to work with (I need to deal with json), and is very
    incomplete. PRs are welcome to improve this interface!

### Progress

At the time of writing, you can join on 1.8, 1.12, and 1.14+. You can interact
with the world, send chat messages, run commands, and interact with other players.
Core concepts like chunk data work well, and are heavily tested.

This is constantly changing, as I don't update this README that much. To see which
versions work, I recommend cloning the project and running it yourself. I may also
host a public demo server in the future, which will be open for anyone to join.

### Architecture

Bamboo uses a proxy-server model. The proxy talks to a Minecraft client over
the TCP based Minecraft protocol, and talks to the server over a custom TCP based
protocol at the same time. This has a number of benefits, such as performance,
scalability, and most importantly, cross-versioning. The Minecraft client changes
its packet definition quite a bit for every version, and supporting all the way
back to 1.8 means that almost everything is different. Having my own proxy in place
means the server works entirely with latest versioned data, and the proxy handles
all the older versions.

As for the server itself, it does a lot of things differently from the vanilla
server. For one, there are very few global locks. This server is designed to be
heavily multithreaded, and to do so, it has a separate tick loop for every
player. Things like redstone, entities moving, and water flowing will be
implemented on a region basis. I do not know how large each of these regions
will be, but they will be small enough that large redstone machines will run on
many different threads at once.

The main goal for speed on the server is for players. I really want to see 1000
players in one server. I was getting there with the previous implementation:
with just a dual-core machine, I was able to support around 150 bots. This is an
exponentially difficult problem, but it mostly comes down to data transfer. So
if all of the proxies are running on separate machines, and if the main server
is on a fast enough machine, it should be possible.

### Modules

Any module on this list depends on at least one of the modules before it in the
list.

 - `bb_macros`: Contains some small utilities used in bb_server.
 - `bb_transfer`: The server-proxy communication protocol. These
   packets describe nothing about there format, so it is up the
   other side to know the protocol spec.
 - `bb_data`: The code generator. This takes prismarine data and
   generates Rust source.
 - `bb_generated`: The output of `bb_data`. This is a separate
   crate to improve rebuild times.
 - `bb_common`: Common utilities. These are things like item
   parsers, UUIDs, etc. This is changed often, so `bb_generated`
   copies some of the code from here.
 - `bb_server`: The Minecraft server. This is the first binary
   target.
 - `bb_proxy`: The proxy. This is required for any clients to
   connection to the server.
 - `bb_cli`: A cli tool, used to connect to a Minecraft server and
   validate that it is sending good data (things like making sure
   the client won't leak chunks, checks for keep alive packets, etc).

### For Rust developers

If you would like to contribute to this project, I welcome your changes! Anything
in the features list above are all good tasks to work on, and would be very appreciated.

If you are looking for the generated protocol code, you can go from this project
directory into the generated code directory using this command:
```bash
# For the proxy in debug mode:
cd $(find target/debug/build/bb_proxy*/out | head -n 1)
# For the proxy in release mode:
cd $(find target/release/build/bb_proxy*/out | head -n 1)
# For the server in debug mode:
cd $(find target/debug/build/bb_server*/out | head -n 1)
# For the server in release mode:
cd $(find target/release/build/bb_server*/out | head -n 1)
```

If you have run the server/proxy in debug/release mode, then the appropriate command
should work. Note that these will all bring you to a directory containing the same
folders.

Inside this output directory, there will be a folder called `protocol`, which has
`cb.rs` and `sb.rs` stored. These are the generated protocol files for clientbound
and serverbound packets.

Because of some compiler flags I have setup in `Cargo.toml`, I recommend setting
your IDE to use a different profile. Any time my IDE builds, I pass the flag
`--profile rust-analyzer` to cargo. This makes code validation much faster, as
the dev profile uses opt-level 2 (instead of the default 0). This is because
terrain generation is terribly slow with opt-level set to 0.
