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
  sync::{mpsc::Sender, Mutex},
  time,
};
use tonic::{Status, Streaming};

use common::{
  math::{ChunkPos, FPos, Pos, PosError},
  net::{cb, Other},
  proto::{player_list, Packet, PlayerList},
  util::{
    chat::{Chat, Color},
    UUID,
  },
  version::BlockVersion,
};

use crate::{block, entity, item, net::Connection, player::Player};
use chunk::MultiChunk;
use gen::WorldGen;

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
  players:          Mutex<HashMap<UUID, Arc<Player>>>,
  eid:              AtomicI32,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
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
}

impl World {
  pub fn new(
    block_converter: Arc<block::TypeConverter>,
    item_converter: Arc<item::TypeConverter>,
    entity_converter: Arc<entity::TypeConverter>,
  ) -> Arc<Self> {
    let world = Arc::new(World {
      chunks: RwLock::new(HashMap::new()),
      players: Mutex::new(HashMap::new()),
      eid: 1.into(),
      block_converter,
      item_converter,
      entity_converter,
      generator: StdMutex::new(WorldGen::new()),
      mspt: 0.into(),
    });
    let w = world.clone();
    tokio::spawn(async move {
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

        header.add("big gaming".into()).color(Color::Blue);
        footer.add("mspt: ".into());
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
    self.players.lock().await.insert(player.id(), player.clone());

    // Network recieving task
    let c = conn.clone();
    let p = player.clone();
    tokio::spawn(async move {
      c.run(&p).await.unwrap();
    });

    // Player tick loop
    let mut int = time::interval(Duration::from_millis(50));
    tokio::spawn(async move {
      info!("{} has logged in", player.username());
      // Player init
      {
        let mut out = cb::Packet::new(cb::ID::Login);
        out.set_int("entity_id", self.eid());
        out.set_byte("game_mode", 1); // Creative
        out.set_byte("difficulty", 1); // Normal
        out.set_byte("dimension", 0); // Overworld
        out.set_str("level_type", "default".into());
        out.set_byte("max_players", 0); // Ignored
        out.set_bool("reduced_debug_info", false); // Don't reduce debug info
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

        let mut info =
          PlayerList { action: player_list::Action::AddPlayer.into(), ..Default::default() };
        for out in self
          .for_players(ChunkPos::new(0, 0), |p| {
            let mut out = cb::Packet::new(cb::ID::NamedEntitySpawn);
            out.set_int("entity_id", p.eid());
            out.set_uuid("player_uuid", p.id());
            let (pos, pitch, yaw) = p.pos_look();
            out.set_double("x", pos.x());
            out.set_double("y", pos.y());
            out.set_double("z", pos.z());
            out.set_float("yaw", yaw);
            out.set_float("pitch", pitch);
            out.set_short("current_item", 0);
            out.set_byte_arr("metadata", p.metadata(player.ver()).serialize());
            info.players.push(player_list::Player {
              uuid:             Some(p.id().as_proto()),
              name:             p.username().into(),
              properties:       vec![],
              gamemode:         1,
              ping:             300,
              has_display_name: false,
              display_name:     "".into(),
            });
            Some(out)
          })
          .await
        {
          conn.send(out).await;
        }
        let mut out = cb::Packet::new(cb::ID::PlayerInfo);
        out.set_other(Other::PlayerList(info)).unwrap();
        conn.send(out).await;
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
      info!("{} has logged out", player.username());
      self.players.lock().await.remove(&player.id());
    });
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
  pub async fn for_players<F, R>(&self, pos: ChunkPos, mut f: F) -> Vec<R>
  where
    F: FnMut(&Player) -> Option<R>,
  {
    let mut out = vec![];
    for p in self.players.lock().await.values() {
      // Only call f in p is in view of pos
      if p.in_view(pos) {
        let v = f(p);
        match v {
          Some(v) => out.push(v),
          None => break,
        }
      }
    }
    out
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
      worlds:           vec![],
    };
    w.add_world();
    w
  }

  pub fn add_world(&mut self) {
    self.worlds.push(World::new(
      self.block_converter.clone(),
      self.item_converter.clone(),
      self.entity_converter.clone(),
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
