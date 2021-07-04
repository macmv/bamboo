mod chunk;
mod gen;

use std::{
  collections::HashMap,
  convert::TryInto,
  sync::{
    atomic::{AtomicI32, AtomicU32, Ordering},
    Arc, Mutex as StdMutex, MutexGuard as StdMutexGuard, RwLock,
  },
  time::{Duration, Instant},
};
use tokio::{
  sync::{mpsc::Sender, Mutex, MutexGuard},
  time,
};
use tonic::{Status, Streaming};

use common::{
  math::{ChunkPos, FPos, Pos, PosError},
  net::{cb, Other},
  proto::{player_list, Packet, PlayerList},
  util::{
    chat::{Chat, Color},
    nbt::{Tag, NBT},
  },
  version::{BlockVersion, ProtocolVersion},
};

use crate::{block, entity, item, net::Connection, player::Player, plugin};
use chunk::MultiChunk;
use gen::WorldGen;

mod players;

pub use players::{PlayersIter, PlayersMap};

// pub struct ChunkRef<'a> {
//   pos:    ChunkPos,
//   // Need to keep this is scope while we mess with the chunk
//   chunks: RwLockReadGuard<'a, HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
// }
//
// impl ChunkRef<'_> {
//   fn lock<'a>(&'a self) -> StdMutexGuard<'a, MultiChunk> {
//     self.chunks.get(&self.pos).unwrap().lock().unwrap()
//   }
// }

pub struct World {
  chunks:           RwLock<HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
  players:          Mutex<PlayersMap>,
  eid:              AtomicI32,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
  generator:        StdMutex<WorldGen>,
  mspt:             AtomicU32,
}

pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds:           Vec<Arc<World>>,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
}

impl World {
  pub fn new(
    block_converter: Arc<block::TypeConverter>,
    item_converter: Arc<item::TypeConverter>,
    entity_converter: Arc<entity::TypeConverter>,
    plugins: Arc<plugin::PluginManager>,
  ) -> Arc<Self> {
    let world = Arc::new(World {
      chunks: RwLock::new(HashMap::new()),
      players: Mutex::new(PlayersMap::new()),
      eid: 1.into(),
      block_converter,
      item_converter,
      entity_converter,
      plugins,
      generator: StdMutex::new(WorldGen::new()),
      mspt: 0.into(),
    });
    let w = world.clone();
    tokio::spawn(async move {
      info!("generating terrain...");
      for x in -10..=10 {
        for z in -10..=10 {
          w.chunk(ChunkPos::new(x, z), |_| {});
        }
      }
      info!("done generating terrain");
      w.global_tick_loop().await;
    });
    world
  }
  async fn global_tick_loop(self: Arc<Self>) {
    let mut int = time::interval(Duration::from_millis(50));
    let mut tick = 0;
    loop {
      int.tick().await;
      if tick % 20 == 0 {
        let mut header = Chat::empty();
        let mut footer = Chat::empty();

        header.add("big gaming\n").color(Color::Blue);
        footer.add("\nmspt: ");
        let mspt = self.mspt.swap(0, Ordering::SeqCst) / 20;
        footer.add(format!("{}", mspt)).color(if mspt > 50 {
          Color::Red
        } else if mspt > 20 {
          Color::Gold
        } else if mspt > 10 {
          Color::Yellow
        } else {
          Color::BrightGreen
        });

        let mut out = cb::Packet::new(cb::ID::PlayerlistHeader);
        out.set_str("header", header.to_json());
        out.set_str("footer", footer.to_json());
        for p in self.players.lock().await.values() {
          p.conn().send(out.clone()).await;
        }
      }
      tick += 1;
    }
  }
  async fn new_player(self: Arc<Self>, player: Player) {
    let conn = player.clone_conn();
    let player = Arc::new(player);
    {
      let mut p = self.players.lock().await;
      if p.contains_key(&player.id()) {
        player.disconnect("Another player with the same id is already connected!").await;
        return;
      }
      p.insert(player.id(), player.clone());
    }

    // Network recieving task
    let c = conn.clone();
    let p = player.clone();
    tokio::spawn(async move {
      c.run(p).await.unwrap();
    });

    // Player tick loop
    tokio::spawn(async move {
      let name = player.username().to_string();
      let id = player.id();
      info!("{} has logged in", name);
      self.player_loop(player, conn).await;
      info!("{} has logged out", name);
      self.players.lock().await.remove(&id);
    });
  }

  async fn player_loop(&self, player: Arc<Player>, conn: Arc<Connection>) {
    let mut int = time::interval(Duration::from_millis(50));
    // Player init
    {
      let mut out = cb::Packet::new(cb::ID::Login);
      out.set_int("entity_id", self.eid());
      out.set_byte("game_mode", 1); // Creative
      out.set_byte("difficulty", 1); // Normal
      if player.ver() < ProtocolVersion::V1_16 {
        out.set_byte("dimension", 0); // Overworld
      }
      out.set_str("level_type", "default".into());
      out.set_byte("max_players", 0); // Ignored
      out.set_bool("reduced_debug_info", false); // Don't reduce debug info

      // 1.13+
      out.set_byte("view_distance", 10); // 10 chunk view distance TODO: Don't hardcode view distance

      // 1.15+
      out.set_byte("hashed_seed", 0);
      out.set_bool("enable_respawn_screen", true);

      // 1.16+
      if player.ver() >= ProtocolVersion::V1_16 {
        out.set_bool("is_hardcore", false);
        out.set_bool("is_flat", false); // Changes the horizon line
        out.set_byte("previous_game_mode", 1);
        out.set_str("world_name", "overworld".into());
        out.set_bool("is_debug", false); // This is not reduced_debug_info, this is for the world being a debug world

        let dimension = Tag::compound(&[
          ("piglin_safe", Tag::Byte(0)),
          ("natural", Tag::Byte(1)),
          ("ambient_light", Tag::Float(0.0)),
          ("fixed_time", Tag::Long(6000)),
          ("infiniburn", Tag::String("".into())),
          ("respawn_anchor_works", Tag::Byte(0)),
          ("has_skylight", Tag::Byte(1)),
          ("bed_works", Tag::Byte(1)),
          ("effects", Tag::String("minecraft:overworld".into())),
          ("has_raids", Tag::Byte(0)),
          ("logical_height", Tag::Int(128)),
          ("coordinate_scale", Tag::Float(1.0)),
          ("ultrawarm", Tag::Byte(0)),
          ("has_ceiling", Tag::Byte(0)),
        ]);
        let biome = Tag::compound(&[
          ("precipitation", Tag::String("rain".into())),
          ("depth", Tag::Float(1.0)),
          ("temperature", Tag::Float(1.0)),
          ("scale", Tag::Float(1.0)),
          ("downfall", Tag::Float(1.0)),
          ("category", Tag::String("none".into())),
          (
            "effects",
            Tag::compound(&[
              ("sky_color", Tag::Int(0xff00ff)),
              ("water_color", Tag::Int(0xff00ff)),
              ("fog_color", Tag::Int(0xff00ff)),
              ("water_fog_color", Tag::Int(0xff00ff)),
            ]),
          ),
        ]);
        let codec = NBT::new(
          "",
          Tag::compound(&[
            (
              "minecraft:dimension_type",
              Tag::compound(&[
                ("type", Tag::String("minecraft:dimension_type".into())),
                (
                  "value",
                  Tag::List(vec![Tag::compound(&[
                    ("name", Tag::String("minecraft:overworld".into())),
                    ("id", Tag::Int(0)),
                    ("element", dimension.clone()),
                  ])]),
                ),
              ]),
            ),
            (
              "minecraft:worldgen/biome",
              Tag::compound(&[
                ("type", Tag::String("minecraft:worldgen/biome".into())),
                (
                  "value",
                  Tag::List(vec![Tag::compound(&[
                    ("name", Tag::String("minecraft:plains".into())),
                    ("id", Tag::Int(0)),
                    ("element", biome),
                  ])]),
                ),
              ]),
            ),
          ]),
        );
        out.set_byte_arr("dimension_codec", codec.serialize());
        out.set_byte_arr("dimension", NBT::new("", dimension).serialize());
        out.set_str_arr("world_names", vec!["minecraft:overworld".into()]);
      }

      conn.send(out).await;

      for x in -10..=10 {
        for z in -10..=10 {
          conn.send(self.serialize_chunk(ChunkPos::new(x, z), player.ver().block())).await;
        }
      }

      let mut out = cb::Packet::new(cb::ID::Position);
      out.set_double("x", 0.0); // X
      out.set_double("y", 60.0); // Y
      out.set_double("z", 0.0); // Z
      out.set_float("yaw", 0.0); // Yaw
      out.set_float("pitch", 0.0); // Pitch
      out.set_byte("flags", 0); // Flags
      out.set_int("teleport_id", 1234); // TP id
      conn.send(out).await;

      let mut info = PlayerList {
        action: player_list::Action::AddPlayer.into(),
        players: vec![player_list::Player {
          uuid:             Some(player.id().as_proto()),
          name:             player.username().into(),
          properties:       vec![],
          gamemode:         1,
          ping:             300, // TODO: Ping
          has_display_name: false,
          display_name:     "".into(),
        }],
        ..Default::default()
      };
      let mut spawn_packets = vec![];
      for other in self.players().await.iter().in_view(ChunkPos::new(0, 0)).not(player.id()) {
        // Add player to the list of players that other knows about
        let mut out = cb::Packet::new(cb::ID::PlayerInfo);
        out
          .set_other(Other::PlayerList(PlayerList {
            action:  player_list::Action::AddPlayer.into(),
            players: vec![player_list::Player {
              uuid:             Some(player.id().as_proto()),
              name:             player.username().into(),
              properties:       vec![],
              gamemode:         1,
              ping:             300, // TODO: Ping
              has_display_name: false,
              display_name:     "".into(),
            }],
          }))
          .unwrap();
        other.conn().send(out).await;
        // Create a packet that will spawn player for other
        let mut out = cb::Packet::new(cb::ID::NamedEntitySpawn);
        out.set_int("entity_id", player.eid());
        out.set_uuid("player_uuid", player.id());
        let (pos, pitch, yaw) = player.pos_look();
        out.set_double("x", pos.x());
        out.set_double("y", pos.y());
        out.set_double("z", pos.z());
        out.set_float("yaw", yaw);
        out.set_float("pitch", pitch);
        out.set_short("current_item", 0);
        out.set_byte_arr("metadata", player.metadata(other.ver()).serialize());
        other.conn().send(out).await;

        // Add other to the list of players that player knows about
        info.players.push(player_list::Player {
          uuid:             Some(other.id().as_proto()),
          name:             other.username().into(),
          properties:       vec![],
          gamemode:         1,
          ping:             300,
          has_display_name: false,
          display_name:     "".into(),
        });
        // Create a packet that will spawn other for player
        let mut out = cb::Packet::new(cb::ID::NamedEntitySpawn);
        out.set_int("entity_id", other.eid());
        out.set_uuid("player_uuid", other.id());
        let (pos, pitch, yaw) = other.pos_look();
        out.set_double("x", pos.x());
        out.set_double("y", pos.y());
        out.set_double("z", pos.z());
        out.set_float("yaw", yaw);
        out.set_float("pitch", pitch);
        out.set_short("current_item", 0);
        out.set_byte_arr("metadata", other.metadata(player.ver()).serialize());
        spawn_packets.push(out);
      }
      // Need to send the player info before the spawn packets
      let mut out = cb::Packet::new(cb::ID::PlayerInfo);
      out.set_other(Other::PlayerList(info)).unwrap();
      conn.send(out).await;
      for p in spawn_packets {
        conn.send(p).await;
      }
    }
    // Player tick loop
    let mut tick = 0;
    loop {
      int.tick().await;
      if conn.closed() {
        // TODO: Close any other tasks for this player
        break;
      }
      let start = Instant::now();
      // Updates the player correctly, and performs collision checks. This also
      // handles new chunks.
      player.tick().await;
      // Do player collision and packets and stuff
      // Once per second, send keep alive packet
      if tick % 20 == 0 {
        let mut out = cb::Packet::new(cb::ID::KeepAlive);
        out.set_int("keep_alive_id", 1234556);
        conn.send(out).await;
      }
      tick += 1;
      self.mspt.fetch_add(start.elapsed().as_millis().try_into().unwrap(), Ordering::SeqCst);
    }
  }

  /// Returns a new, unique EID.
  pub fn eid(&self) -> i32 {
    self.eid.fetch_add(1, Ordering::SeqCst)
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn get_block_converter(&self) -> &block::TypeConverter {
    &self.block_converter
  }

  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn get_item_converter(&self) -> &item::TypeConverter {
    &self.item_converter
  }

  /// Returns the current entity converter. This can be used to convert old
  /// entity ids to new ones, and vice versa.
  pub fn get_entity_converter(&self) -> &entity::TypeConverter {
    &self.entity_converter
  }

  /// Returns the plugin manager. This is how events can be sent to plugins.
  pub fn get_plugins(&self) -> &plugin::PluginManager {
    &self.plugins
  }

  /// This calls f(), and passes it a locked chunk. This will also generate a
  /// new chunk if there is not one stored there.
  ///
  /// I tried to make the chunk a returned value, but that ended up being too
  /// difficult. Since the entire chunks map must be locked for reading, that
  /// read lock must be held while the chunk is in scope. Because of this, you
  /// would have needed to call two functions to get it working. I tried my best
  /// with the [`Deref`](std::ops::Deref) trait, but I couldn't get it to work
  /// the way I liked.
  pub fn chunk<F, R>(&self, pos: ChunkPos, f: F) -> R
  where
    F: FnOnce(StdMutexGuard<MultiChunk>) -> R,
  {
    // We first check (read-only) if we need to generate a new chunk
    if !self.chunks.read().unwrap().contains_key(&pos) {
      // If we do, we lock it for writing
      let mut chunks = self.chunks.write().unwrap();
      // Make sure that the chunk was not written in between locking this chunk
      chunks.entry(pos).or_insert_with(|| {
        let mut c = MultiChunk::new(self.block_converter.clone());
        self.generator.lock().unwrap().generate(pos, &mut c);
        Arc::new(StdMutex::new(c))
      });
    }
    let chunks = self.chunks.read().unwrap();
    let c = chunks[&pos].lock().unwrap();
    f(c)
  }

  /// This serializes a chunk for the given version. This packet can be sent
  /// directly to a client. Note that on most vanilla versions, sending a chunk
  /// to a client that already has loaded that chunk will cause a memory leak.
  /// Unloading a chunk multiple times will not cause a memory leak. If you are
  /// trying to re-send an entire chunk to a player, make sure to send them an
  /// unload chunk packet first. Use at your own risk!
  pub fn serialize_chunk(&self, pos: ChunkPos, ver: BlockVersion) -> cb::Packet {
    let mut out = cb::Packet::new(cb::ID::MapChunk);
    self.chunk(pos, |c| {
      let mut pb = c.to_proto(ver);
      pb.x = pos.x();
      pb.z = pos.z();
      out.set_other(Other::Chunk(pb)).unwrap();
    });
    out
  }

  /// This sets a block within the world. It will return an error if the
  /// position is outside of the world.
  pub async fn set_block(&self, pos: Pos, ty: &block::Type) -> Result<(), PosError> {
    self.chunk(pos.chunk(), |mut c| c.set_type(pos.chunk_rel(), ty))?;

    for p in self.players.lock().await.values() {
      let mut out = cb::Packet::new(cb::ID::BlockChange);
      out.set_pos("location", pos);
      out.set_int("type", self.block_converter.to_old(ty.id(), p.ver().block()) as i32);
      p.conn().send(out).await;
    }
    Ok(())
  }

  /// This sets a block within the world. This will use the default type of the
  /// given kind. It will return an error if the position is outside of the
  /// world.
  pub async fn set_kind(&self, pos: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.set_block(pos, self.block_converter.get(kind).default_type()).await
  }

  /// This broadcasts a chat message to everybody in the world.
  pub async fn broadcast(&self, msg: &Chat) {
    let mut out = cb::Packet::new(cb::ID::Chat);
    out.set_str("message", msg.to_json());
    out.set_byte("position", 0); // Chat box, not over hotbar

    for p in self.players.lock().await.values() {
      p.conn().send(out.clone()).await;
    }
  }

  // Runs f for all players within render distance of the chunk.
  pub async fn players(&self) -> MutexGuard<'_, PlayersMap> {
    self.players.lock().await
  }
}

impl Default for WorldManager {
  fn default() -> Self {
    WorldManager::new()
  }
}

impl WorldManager {
  pub fn new() -> Self {
    let mut w = WorldManager {
      block_converter:  Arc::new(block::TypeConverter::new()),
      item_converter:   Arc::new(item::TypeConverter::new()),
      entity_converter: Arc::new(entity::TypeConverter::new()),
      plugins:          Arc::new(plugin::PluginManager::new()),
      worlds:           vec![],
    };
    w.add_world();
    w
  }

  pub async fn run(&self, wm: Arc<WorldManager>) {
    self.plugins.clone().run(wm).await;
  }

  /// Adds a new world. Currently, this requires a mutable reference, which
  /// cannot be obtained outside of initialization.
  pub fn add_world(&mut self) {
    self.worlds.push(World::new(
      self.block_converter.clone(),
      self.item_converter.clone(),
      self.entity_converter.clone(),
      self.plugins.clone(),
    ));
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn get_block_converter(&self) -> &block::TypeConverter {
    &self.block_converter
  }

  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn get_item_converter(&self) -> &item::TypeConverter {
    &self.item_converter
  }

  /// Broadcasts a message to everyone one the server.
  pub async fn broadcast(&self, msg: &Chat) {
    // info!("BROADCASTING");
    for w in &self.worlds {
      w.broadcast(msg).await;
    }
  }

  /// Adds a new player into the game. This should be called when a new grpc
  /// proxy connects.
  pub async fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    let conn = Arc::new(Connection::new(req, tx));
    let (username, uuid, ver) = conn.wait_for_login().await;
    let w = self.worlds[0].clone();
    let player =
      Player::new(w.eid(), username, uuid, conn, ver, w.clone(), FPos::new(0.0, 60.0, 0.0));
    w.new_player(player).await;
  }
}
