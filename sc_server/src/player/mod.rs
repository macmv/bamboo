use crate::{
  command::CommandSender,
  entity::Metadata,
  item::{Inventory, Stack},
  net::ConnSender,
  world::World,
};
use sc_common::{
  math::{ChunkPos, FPos, Pos, PosError, Vec3},
  net::cb,
  util::{Chat, UUID},
  version::ProtocolVersion,
};
use std::{
  f64::consts,
  fmt,
  sync::{Arc, Mutex, MutexGuard},
};

mod tick;

#[derive(Debug, Clone)]
struct PlayerPosition {
  // This is the current position of the player. It is only updated once per tick.
  curr: FPos,

  // This is the position on the previous tick. It is only updated once per tick.
  prev: FPos,

  // This is the most recently recieved position packet. It is updated whenever a position packet
  // is recieved. It is also used to set x,y,z on the next tick.
  next: FPos,

  yaw:   f32,
  pitch: f32,

  next_yaw:   f32,
  next_pitch: f32,
}

#[derive(Debug)]
pub struct PlayerInventory {
  inv:            Inventory,
  // An index into the hotbar (0..=8)
  selected_index: u8,
}

impl PlayerInventory {
  pub fn new() -> Self {
    PlayerInventory { inv: Inventory::new(46), selected_index: 0 }
  }

  /// Returns the item in the player's main hand.
  pub fn main_hand(&self) -> &Stack {
    self.inv.get(self.selected_index as u32 + 36)
  }

  /// Returns the currently selected hotbar index.
  pub fn selected_index(&self) -> u8 {
    self.selected_index
  }

  /// Sets the selected index. Should only be used when recieving a held item
  /// slot packet.
  pub(crate) fn set_selected(&mut self, index: u8) {
    self.selected_index = index;
  }

  /// Gets the item at the given index. 0 is part of the armor slots, not the
  /// start of the hotbar. To access the hotbar, add 36 to the index returned
  /// from main_hand.
  pub fn get(&self, index: u32) -> &Stack {
    self.inv.get(index)
  }

  /// Sets the item in the inventory.
  ///
  /// TODO: Send a packet here.
  pub fn set(&mut self, index: u32, stack: Stack) {
    self.inv.set(index, stack)
  }
}

pub struct Player {
  // The EID of the player. Never changes.
  eid:           i32,
  // Player's username
  username:      String,
  uuid:          UUID,
  conn:          ConnSender,
  ver:           ProtocolVersion,
  world:         Arc<World>,
  view_distance: u32,

  inv: Mutex<PlayerInventory>,

  pos: Mutex<PlayerPosition>,
}

impl fmt::Debug for Player {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Player")
      .field("username", &self.username)
      .field("uuid", &self.uuid)
      .field("ver", &self.ver)
      .field("view_distance", &self.view_distance)
      .field("inv", &self.inv)
      .field("pos", &self.pos)
      .finish()
  }
}

impl Player {
  pub fn new(
    eid: i32,
    username: String,
    uuid: UUID,
    conn: ConnSender,
    ver: ProtocolVersion,
    world: Arc<World>,
    pos: FPos,
  ) -> Self {
    Player {
      eid,
      username,
      uuid,
      conn,
      ver,
      world,
      view_distance: 10,
      inv: Mutex::new(PlayerInventory::new()),
      pos: Mutex::new(PlayerPosition {
        curr:       pos,
        prev:       pos,
        next:       pos,
        yaw:        0.0,
        pitch:      0.0,
        next_yaw:   0.0,
        next_pitch: 0.0,
      }),
    }
  }

  /// Returns the player's username.
  pub fn username(&self) -> &str {
    &self.username
  }

  /// Returns the player's entity id. Used to send packets about entities.
  pub fn eid(&self) -> i32 {
    self.eid
  }
  /// Returns the player's uuid. Used to lookup players in the world.
  pub fn id(&self) -> UUID {
    self.uuid
  }

  /// Returns the version that this client connected with. This will only change
  /// if the player disconnects and logs in with another client.
  pub fn ver(&self) -> ProtocolVersion {
    self.ver
  }

  /// Returns a locked reference to the player's inventory.
  pub fn lock_inventory(&self) -> MutexGuard<PlayerInventory> {
    self.inv.lock().unwrap()
  }

  /// Returns a reference to the world the player is in.
  pub fn world(&self) -> &Arc<World> {
    &self.world
  }

  /// This will move the player on the next player tick. Used whenever a
  /// position packet is recieved.
  pub(crate) fn set_next_pos(&self, x: f64, y: f64, z: f64) {
    let mut pos = self.pos.lock().unwrap();
    pos.next = FPos::new(x, y, z);
  }

  /// This will set the player's look direction on the next player tick. Used
  /// whenever a player look packet is recieved.
  pub(crate) fn set_next_look(&self, yaw: f32, pitch: f32) {
    let mut pos = self.pos.lock().unwrap();
    pos.next_yaw = yaw;
    pos.next_pitch = pitch;
  }

  /// Sends the player a chat message.
  pub fn send_message(&self, msg: &Chat) {
    self.send(cb::Packet::Chat {
      message:      msg.to_json(),
      position:     0, // Chat box, not system message or over hotbar
      sender_v1_16: Some(self.id()),
    });
  }
  /// Sends the player a chat message, which will appear over their hotbar.
  pub fn send_hotbar(&self, msg: &Chat) {
    self.send(cb::Packet::Chat {
      message:      msg.to_json(),
      position:     2, // Hotbar, not chat box or system message
      sender_v1_16: Some(self.id()),
    });
  }
  /// Disconnects the player. The given chat message will be shown on the
  /// loading screen.
  ///
  /// This may not have an effect immediately. This only sends a disconnect
  /// packet. Assuming normal operation, the client will then disconnect after
  /// they have recieved this packet.
  ///
  /// TODO: This should terminate the connection after this packet is sent.
  /// Closing the channel will drop the packet before it can be sent, so we need
  /// some other way of closing it later.
  pub fn disconnect<C: Into<Chat>>(&self, msg: C) {
    self.send(cb::Packet::KickDisconnect { reason: msg.into().to_json() });
  }

  /// Generates the player's metadata for the given version. This will include
  /// all fields possible about the player. This should only be called when
  /// spawning in a new player.
  pub fn metadata(&self, ver: ProtocolVersion) -> Metadata {
    let meta = Metadata::new(ver);
    // meta.set_byte(0, 0b00000000).unwrap();
    meta
  }

  /// Returns the player's position. This is only updated once per tick. This
  /// also needs to lock a mutex, so you should not call it very often.
  pub fn pos(&self) -> FPos {
    let pos = self.pos.lock().unwrap();
    pos.curr
  }
  /// Returns the player's block position. This is the block that their feet are
  /// in. This is the same thing as calling [`p.pos().block()`](Self::pos).
  fn block_pos(&self) -> Pos {
    self.pos().block()
  }
  /// Returns the player's position and looking direction. This is only updated
  /// once per tick. This also locks a mutex, so you should not call it very
  /// often.
  pub fn pos_look(&self) -> (FPos, f32, f32) {
    let pos = self.pos.lock().unwrap();
    (pos.curr, pos.pitch, pos.yaw)
  }
  /// Returns the player's current and previous position. This is only updated
  /// once per tick. This needs to lock a mutex, so if you need the player's
  /// previous position, it is better to call this without calling
  /// [`pos`](Self::pos). The first item returned is the current position, and
  /// the second item is the previous position.
  pub fn pos_with_prev(&self) -> (FPos, FPos) {
    let pos = self.pos.lock().unwrap();
    (pos.curr, pos.prev)
  }

  /// Returns the player's pitch and yaw angle. This is the amount that they are
  /// looking to the side. It is in the range -180..180. This is only updated
  /// once per tick.
  pub fn look(&self) -> (f32, f32) {
    let pos = self.pos.lock().unwrap();
    (pos.pitch, pos.yaw)
  }

  /// Returns a unit vector which is the direction this player is facing.
  pub fn look_as_vec(&self) -> Vec3 {
    let (pitch, yaw) = self.look();
    let pitch = (pitch as f64) / 180.0 * consts::PI;
    let yaw = (yaw as f64) / 180.0 * consts::PI;
    let m = pitch.cos();
    // The coordinate system of minecraft means that we need to do this hell to get
    // the axis to line up correctly.
    Vec3::new(-yaw.sin() * m, -pitch.sin(), yaw.cos() * m)
  }

  /// Returns true if the player is within render distance of the given chunk
  pub fn in_view(&self, pos: ChunkPos) -> bool {
    let delta = pos - self.pos().block().chunk();
    // TODO: Store view distance
    delta.x().abs() <= 10 && delta.z().abs() <= 10
  }

  /// Sets the player's fly speed. Unlike the packet, this is a multipler. So
  /// setting their flyspeed to 1.0 is the default speed.
  pub fn set_flyspeed(&self, speed: f32) {
    self.send(cb::Packet::Abilities {
      // 0x01: No damage
      // 0x02: Flying
      // 0x04: Can fly
      // 0x08: Can instant break
      flags:         0x02 | 0x04 | 0x08,
      flying_speed:  speed * 0.05,
      walking_speed: 0.1,
    });
  }

  /// Sends a block update packet for the block at the given position. This
  /// ensures that the client sees what the server sees at that position.
  ///
  /// This is mostly used for placing blocks. If you place a block on a stone
  /// block, then the position you clicked on is not the same as the position
  /// where the new block is. However, if you click on tall grass, then the tall
  /// grass will be replaced by the new block. The client assumes this, and it
  /// ends up becoming desyncronized from the server. So this function is called
  /// on that tall grass block, to prevent the client from showing the wrong
  /// block.
  pub fn sync_block_at(&self, pos: Pos) -> Result<(), PosError> {
    let ty = self.world().get_block(pos)?;
    self.send(cb::Packet::BlockChange {
      location: pos,
      type_:    self.world().block_converter().to_old(ty.id(), self.ver().block()) as i32,
    });
    Ok(())
  }

  /// Sends the given packet to this player. This will be flushed as soon as the
  /// outgoing buffer as space, which is immediately in most situations. If a
  /// bunch of data is being sent at once, this function can block. So this
  /// technically can result in deadlocks, but the way the threads are setup
  /// right now mean that no channel will block another channel, so in practice
  /// this will only produce slow downs, never deadlocks.
  pub fn send(&self, p: cb::Packet) {
    self.conn.send(p);
  }

  /// Returns true if the player's connection is closed.
  pub fn closed(&self) -> bool {
    // TODO: Hold onto an Arc<AtomicBool>
    false
  }
}

impl CommandSender for Player {
  fn block_pos(&self) -> Option<Pos> {
    Some(self.block_pos())
  }
}

#[test]
fn assert_sync() {
  fn is_sync<T: Send + Sync>() {}
  is_sync::<Player>(); // only compiles if player is Sync
}
