use crate::{
  block,
  command::CommandSender,
  entity,
  entity::EntityRef,
  item::{Inventory, Stack},
  net::ConnSender,
  world::World,
};
use bb_common::{
  math::{ChunkPos, FPos, Pos, PosError, Vec3},
  metadata::Metadata,
  net::{cb, sb::PlayerCommand},
  util::{Chat, GameMode, JoinInfo, UUID},
  version::ProtocolVersion,
};
use parking_lot::{Mutex, MutexGuard};
use std::{f64::consts, fmt, net::SocketAddr, sync::Arc, time::Instant};
use std::ops::{Add, Mul};

mod inventory;
mod scoreboard;
mod team;
mod tick;

pub use inventory::PlayerInventory;
pub use scoreboard::Scoreboard;
pub use team::Team;

#[derive(Debug, Clone)]
struct DigProgress {
  progress:     f64,
  pos:          Pos,
  kind:         block::Kind,
  /// If we receive a `DigStatus::Finish` packet before the server thinks this
  /// is done, we set this to `true`. Then, in the tick loop, we still track
  /// progress, and send an update back once we think it should be finished.
  ///
  /// This will prevent digging too fast, while also preventing desyncs between
  /// the client and server.
  wants_finish: bool,
}

#[derive(Debug, Clone)]
struct PlayerPosition {
  // This is the current position of the player. It is only updated once per tick.
  curr: FPos,

  // This is the position on the previous tick. It is only updated once per tick.
  prev: FPos,

  // This is the most recently recieved position packet. It is updated whenever a position packet
  // is recieved. It is also used to set x,y,z on the next tick.
  next: FPos,

  vel: Vec3,

  yaw:   f32,
  pitch: f32,

  next_yaw:   f32,
  next_pitch: f32,

  last_set_pos: Instant,

  crouching: bool,
  sprinting: bool,
  swimming:  bool,

  /// Stores the current progress. Mining time can change between when you start
  /// and when you end, so the most accurate way to track mining time is to
  /// increase this by some fraction every tick, and then check if its >= 1 when
  /// they send a `DigStatus::Finish` packet.
  ///
  /// Will always be `None` outside of survival.
  dig_progress: Option<DigProgress>,
}

#[derive(Debug, Clone)]
struct PlayerHealth {
  health:     f32,
  absorption: f32,
}

#[derive(Debug, Clone)]
struct PlayerFood {
  food:       i32,
  saturation: f32,
}

pub struct Player {
  // The EID of the player. Never changes.
  eid:           i32,
  username:      String,
  tab_name:      Mutex<Option<Chat>>,
  uuid:          UUID,
  conn:          ConnSender,
  ver:           ProtocolVersion,
  world:         Arc<World>,
  view_distance: u32,

  game_mode: Mutex<GameMode>,

  inv:        Mutex<PlayerInventory>,
  scoreboard: Mutex<Scoreboard>,
  pos:        Mutex<PlayerPosition>,

  health: Mutex<PlayerHealth>,
  food:   Mutex<PlayerFood>,
}

impl fmt::Debug for Player {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Player")
      .field("username", &self.username)
      .field("uuid", &self.uuid)
      .field("ver", &self.ver)
      .field("view_distance", &self.view_distance)
      .field("inv", &self.inv)
      .field("scoreboard", &self.scoreboard)
      .field("pos", &self.pos)
      .field("health", &self.health)
      .finish()
  }
}

impl Drop for Player {
  fn drop(&mut self) {
    info!("dropping player {}", self.username);
  }
}

impl Player {
  pub fn new(
    eid: i32,
    conn: ConnSender,
    info: JoinInfo,
    world: Arc<World>,
    pos: FPos,
  ) -> Arc<Self> {
    Arc::new_cyclic(|weak| Player {
      eid,
      username: info.username,
      tab_name: Mutex::new(None),
      uuid: info.uuid,
      inv: PlayerInventory::new(weak.clone(), conn.clone()).into(),
      scoreboard: Scoreboard::new(conn.clone()).into(),
      conn,
      ver: ProtocolVersion::from(info.ver as i32),
      view_distance: world.config().get("view-distance"),
      game_mode: Mutex::new(world.world_manager().default_game_mode()),
      world,
      pos: PlayerPosition {
        curr:         pos,
        prev:         pos,
        next:         pos,
        vel:          Vec3::new(0.0, 0.0, 0.0),
        yaw:          0.0,
        pitch:        0.0,
        next_yaw:     0.0,
        next_pitch:   0.0,
        last_set_pos: Instant::now(),
        crouching:    false,
        sprinting:    false,
        swimming:     false,
        dig_progress: None,
      }
      .into(),
      health: PlayerHealth { health: 20.0, absorption: 0.0 }.into(),
      food: PlayerFood { food: 20, saturation: 5.0 }.into(),
    })
  }

  /// Returns the player's username.
  pub fn username(&self) -> &String { &self.username }

  /// Returns the player's entity id. Used to send packets about entities.
  pub fn eid(&self) -> i32 { self.eid }
  /// Returns the player's uuid. Used to lookup players in the world.
  pub fn id(&self) -> UUID { self.uuid }
  /// Returns the player's view disstance. This is how far they can see in
  /// chunks.
  pub fn view_distance(&self) -> u32 { self.view_distance }

  pub fn lock_health(&self) -> MutexGuard<'_, PlayerHealth> { self.health.lock() }

  pub fn health(&self) -> f32 { self.health.lock().health }

  /// Returns the version that this client connected with. This will only change
  /// if the player disconnects and logs in with another client.
  pub fn ver(&self) -> ProtocolVersion { self.ver }

  /// Returns a locked reference to the player's inventory.
  pub fn lock_inventory(&self) -> MutexGuard<PlayerInventory> { self.inv.lock() }
  /// Returns a locked reference to the player's scoreboard.
  pub fn lock_scoreboard(&self) -> MutexGuard<Scoreboard> { self.scoreboard.lock() }

  /// Returns a reference to the world the player is in.
  pub fn world(&self) -> &Arc<World> { &self.world }

  /// This will move the player on the next player tick. Used whenever a
  /// position packet is recieved.
  pub(crate) fn set_next_pos(&self, x: f64, y: f64, z: f64) {
    let mut pos = self.pos.lock();
    pos.next = FPos::new(x, y, z);
  }

  /// This will set the player's look direction on the next player tick. Used
  /// whenever a player look packet is recieved.
  pub(crate) fn set_next_look(&self, yaw: f32, pitch: f32) {
    let mut pos = self.pos.lock();
    pos.next_yaw = yaw;
    pos.next_pitch = pitch;
  }

  /// Teleports the player to the given position, and set's their looking
  /// direction to the given yaw/pitch. You cannot teleport a player without
  /// also setting their yaw/pitch.
  pub fn teleport(&self, pos: FPos, yaw: f32, pitch: f32) {
    self.send(cb::Packet::SetPosLook {
      pos,
      yaw,
      pitch,
      flags: 0,
      teleport_id: 0,
      should_dismount: true,
    });
    let mut p = self.pos.lock();
    p.next_yaw = yaw;
    p.next_pitch = pitch;
    p.next = pos;
  }

  /// Sends the player a chat message.
  pub fn send_message(&self, msg: Chat) {
    self.send(cb::Packet::Chat {
      msg,
      ty: 0, // Chat box, not system message or over hotbar
    });
  }
  /// Sends the player a chat message, which will appear over their hotbar.
  pub fn send_hotbar(&self, msg: Chat) {
    self.send(cb::Packet::Chat {
      msg,
      ty: 2, // Hotbar, not chat box or system message
    });
  }

  /// Sets the title for this player. To show the title and subtitle, call
  /// [`show_title`](Self::show_title).
  pub fn set_title(&self, title: Chat) {
    self.send(cb::Packet::Title { action: cb::TitleAction::Title(title) });
  }
  /// Sets the subtitle for this player. To show the title and subtitle, call
  /// [`show_title`](Self::show_title).
  pub fn set_subtitle(&self, subtitle: Chat) {
    self.send(cb::Packet::Title { action: cb::TitleAction::Subtitle(subtitle) });
  }
  /// Shows the current title to the player. The `fade_in`, `stay`, and
  /// `fade_out` arguments are all in ticks.
  pub fn show_title(&self, fade_in: u32, stay: u32, fade_out: u32) {
    self.send(cb::Packet::Title { action: cb::TitleAction::Times { fade_in, stay, fade_out } });
  }

  pub fn show_inventory(&self, inv: Inventory<27>, title: &Chat) {
    let ty = (inv.size() / 9) as u8;
    self.send(cb::Packet::WindowOpen { wid: 1, ty, title: title.to_json() });
    self.send(cb::Packet::WindowItems {
      wid:   1,
      items: inv.items().iter().map(|i| i.to_item()).collect(),
      held:  Stack::empty().to_item(),
    });
    self.lock_inventory().open_window(inv);
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
  pub fn disconnect<C: Into<Chat>>(&self, _msg: C) {
    // self.send(cb::Packet::KickDisconnect { reason: msg.into().to_json() });
    self.remove();
  }

  /// Disconnects the player, without sending a disconnect message. Prefer
  /// [`disconnect`](Self::disconnect) in most situations.
  ///
  /// This is used when a player disconnects on their own, and they need to be
  /// removed from the players list in the world.
  pub(crate) fn remove(&self) { self.world.world_manager().remove_player(self.uuid); }

  /// Returns the status byte for entity metadata. The bits are as follows:
  ///
  /// - `0x01`: Is on fire
  /// - `0x02`: Is crouching
  /// - `0x04`: Only on old versions; is riding
  /// - `0x08`: Is sprinting
  /// - `0x10`: Is swimming
  /// - `0x20`: Is invisible
  /// - `0x40`: Is glowing
  /// - `0x80`: Is flying with elytra
  pub fn status_byte(&self) -> i8 {
    let pos = self.pos.lock();
    ((pos.crouching as i8) << 1) | ((pos.sprinting as i8) << 4) | ((pos.swimming as i8) << 5)
  }

  /// Generates the player's metadata for the given version. This will include
  /// all fields possible about the player. This should only be called when
  /// spawning in a new player.
  pub fn metadata(&self) -> Metadata {
    let mut meta = Metadata::new();
    meta.set_byte(0, self.status_byte());
    // Custom names aren't present for players, so we don't set fields 2 and 3.
    meta
  }

  /// Returns the player's position. This is only updated once per tick. This
  /// also needs to lock a mutex, so you should not call it very often.
  pub fn pos(&self) -> FPos {
    let pos = self.pos.lock();
    pos.curr
  }

  /// Returns if player is currently alive
  pub fn alive(&self) -> bool { (*self.health.lock()).health > 0.0 }

  /// Returns the player's block position. This is the block that their feet are
  /// in. This is the same thing as calling [`p.pos().block()`](Self::pos).
  fn block_pos(&self) -> Pos { self.pos().block() }
  /// Returns the player's position and looking direction. This is only updated
  /// once per tick. This also locks a mutex, so you should not call it very
  /// often.
  pub fn pos_look(&self) -> (FPos, f32, f32) {
    let pos = self.pos.lock();
    (pos.curr, pos.pitch, pos.yaw)
  }
  /// Returns the player's current and previous position. This is only updated
  /// once per tick. This needs to lock a mutex, so if you need the player's
  /// previous position, it is better to call this without calling
  /// [`pos`](Self::pos). The first item returned is the current position, and
  /// the second item is the previous position.
  pub fn pos_with_prev(&self) -> (FPos, FPos) {
    let pos = self.pos.lock();
    (pos.curr, pos.prev)
  }

  /// Returns the player's pitch and yaw angle. This is the amount that they are
  /// looking to the side. It is in the range -180..180. This is only updated
  /// once per tick.
  pub fn look(&self) -> (f32, f32) {
    let pos = self.pos.lock();
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
    delta.x().abs() as u32 <= self.view_distance && delta.z().abs() as u32 <= self.view_distance
  }

  /// Sets the player's fly speed. This is a speed multiplier. So a value of
  /// `1.0` will reset their fly speed to the default.
  pub fn set_flyspeed(&self, speed: f32) {
    self.send(cb::Packet::Abilities {
      invulnerable: false,
      flying:       true,
      allow_flying: true,
      insta_break:  true,
      fly_speed:    speed,
      walk_speed:   1.0,
    });
  }
  /// Sets the player's tab list name. If `name` is `None`, then the display
  /// name will be removed, and the username will show instead.
  ///
  /// Note that this does not update the name above the player's head. The only
  /// way to do that is by adding this player to a team.
  ///
  /// This will produce inconsistent behavior if the player is on a team. Only
  /// use if needed. Using teams is going to be more reliable.
  pub fn set_tab_name(&self, name: Option<Chat>) {
    *self.tab_name.lock() = name.clone();
    let update = cb::Packet::PlayerList {
      action: cb::PlayerListAction::UpdateDisplayName(vec![cb::PlayerListDisplay {
        id:           self.id(),
        display_name: name,
      }]),
    };
    for w in self.world().world_manager().worlds().iter() {
      for p in w.players().iter() {
        p.send(update.clone());
      }
    }
  }
  /// Returns the current tab list name.
  pub fn tab_name(&self) -> MutexGuard<'_, Option<Chat>> { self.tab_name.lock() }

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
    self.send(cb::Packet::BlockUpdate {
      pos,
      state: self.world().block_converter().to_old(ty.id(), self.ver().block()),
    });
    Ok(())
  }

  /// Sends the given packet to this player. This will be flushed as soon as the
  /// outgoing buffer as space, which is immediately in most situations. If a
  /// bunch of data is being sent at once, this function can block. So this
  /// technically can result in deadlocks, but the way the threads are setup
  /// right now mean that no channel will block another channel, so in practice
  /// this will only produce slow downs, never deadlocks.
  pub fn send(&self, p: cb::Packet) { self.conn.send(p); }

  /// Sends the given packet to all players in view of this player, *not
  /// including* `self`. If you want to also send the packet to `self`,
  /// call [`send_all_in_view`](Self::send_all_in_view).
  pub fn send_to_in_view(&self, p: cb::Packet) {
    for other in self.world.players().iter().in_view(self.pos().chunk()).not(self.uuid) {
      other.send(p.clone());
    }
  }
  /// Sends the given packet to all players in view of this player, *including*
  /// `self`. If you don't wnat to send the packet to `self`, call
  /// [`send_to_in_view`](Self::send_to_in_view).
  pub fn send_all_in_view(&self, p: cb::Packet) {
    for other in self.world.players().iter().in_view(self.pos().chunk()) {
      other.send(p.clone());
    }
  }

  /// Returns true if the player's connection is closed.
  pub fn closed(&self) -> bool {
    // TODO: Hold onto an Arc<AtomicBool>
    false
  }

  /// Returns the player's game mode. To change their game mode, call
  /// [`set_game_mode`](Self::set_game_mode).
  pub fn game_mode(&self) -> GameMode { *self.game_mode.lock() }

  /// Updates the player's game mode. This can be retrieved with
  /// [`game_mode`](Self::game_mode).
  pub fn set_game_mode(&self, mode: GameMode) {
    self.send(cb::Packet::ChangeGameState { action: cb::ChangeGameState::GameMode(mode) });
    *self.game_mode.lock() = mode;
  }

  pub fn get_vel(&self) -> Vec3 {
    self.pos.lock().vel
  }

  pub fn set_vel(&self, vel: Vec3) {
    self.pos.lock().vel = vel;
    self.send_vel(vel);
  }

  pub fn add_vel(&self, vel: Vec3) {
    let mut pos = self.pos.lock();
    pos.vel = pos.vel.add(vel);
    self.send_vel(vel);
  }

  fn send_vel(&self, vel: Vec3) {
    self.send_all_in_view(cb::Packet::EntityVelocity {
      eid: self.eid(),
      x:   vel.fixed_x(),
      y:   vel.fixed_y(),
      z:   vel.fixed_z(),
    });
  }

  /// Sends a server switch packet to the proxy. If the ip address is valid, the
  /// proxy will move this player and disconnect them from this server. If the
  /// `ip` is invalid, the proxy will log an error, and the player will not be
  /// moved.
  ///
  /// Because this is all over the network, the player can be disconnected at
  /// any time. If the `ips` was all bad addresses, or the server refused to
  /// accept the connection, the packet `SwitchServerFailed` will be sent back
  /// to us.
  ///
  /// The order of `ips` matters. The first one will be tried first, then the
  /// second one, etc. This is expected to be the result of
  /// [`ToSocketAddrs`](std::net::ToSocketAddrs).
  pub fn switch_to(&self, ips: Vec<SocketAddr>) { self.send(cb::Packet::SwitchServer { ips }); }

  pub(super) fn handle_command(&self, command: PlayerCommand) {
    match command {
      PlayerCommand::StartSprint => self.pos.lock().sprinting = true,
      PlayerCommand::StopSprint => self.pos.lock().sprinting = false,
      PlayerCommand::StartSneak => self.pos.lock().crouching = true,
      PlayerCommand::StopSneak => self.pos.lock().crouching = false,
      _ => {}
    }
    let mut meta = Metadata::new();
    meta.set_byte(0, self.status_byte());
    self.send_to_in_view(cb::Packet::EntityMetadata {
      eid: self.eid(),
      ty: entity::Type::Player.id(),
      meta,
    });
  }

  pub(super) fn set_crouching(&self, crouching: bool) {
    let mut p = self.pos.lock();
    let needs_update = p.crouching != crouching;
    p.crouching = crouching;
    drop(p);
    if needs_update {
      let mut meta = Metadata::new();
      meta.set_byte(0, self.status_byte());
      self.send_to_in_view(cb::Packet::EntityMetadata {
        eid: self.eid(),
        ty: entity::Type::Player.id(),
        meta,
      });
    }
  }

  pub(super) fn attack(&self, other: EntityRef) {
    // Handles base damage and enchantments
    let damage = self.lock_inventory().main_hand().attack_damage();
    // TODO: Strength
    let v = self.look_as_vec().mul(0.4);
    other.damage(damage, true, Vec3::new(v.x, 0.4, v.z));
  }

  /// Returns true if the player can be damaged. This will return `false` if
  /// they are in creative, or are dead.
  ///
  /// This will add more checks in the future.
  pub fn damageable(&self) -> bool {
    // TODO: Void damage
    matches!(self.game_mode(), GameMode::Survival | GameMode::Adventure) && self.alive()
  }

  /// Damages the player. If `blockable` is true, then shields, armor, and
  /// absorption will affect the amount of damage. If `blockable` is false, then
  /// this will deal exactly `damage` amount to the player.
  ///
  /// Returns if the entity has been successfully damaged. 0 damage will still
  /// return `true`. This will only return `false` if [`Player::damageable`] is
  /// `false`.
  pub fn damage(&self, mut amount: f32, blockable: bool, knockback: Vec3) -> bool {
    if !self.damageable() {
      return false;
    }

    if blockable {
      // TODO: Blocking
      /*
      if self.is_blocking() {
        damage = (1.0 + damage) * 0.5;
      }
      */
      // TODO: Armor damage and armor reducing damage
      /*
      let armor_damage = 25.0 - other.armor_total();
      damage = (damage * armor_damage) / 25.0;
       */
    }
    // Here, vanilla would send a metadata update for self's health. This
    // doesn't make any sense, as other players (with a hacked client) can see
    // our health. So, I simply don't send the health to other clients here.
    {
      let mut health = self.health.lock();
      let food = self.food.lock();

      let amount_original = amount;

      amount -= health.absorption;
      if amount < 0.0 {
        amount = 0.0;
      }
      health.absorption -= amount_original - amount;
      if amount != 0.0 {
        let h = health.health;
        health.health = h - amount;
        health.absorption -= amount;
      }

      self.send(cb::Packet::UpdateHealth {
        health:     health.health,
        food:       food.food,
        saturation: food.saturation,
      });

      self.add_vel(knockback);
    }
    let pos = self.pos();
    for p in self.world().players().iter().in_view(self.pos().chunk()) {
      if p.id() != self.id() {
        p.send(cb::Packet::Animation { eid: self.eid(), kind: cb::Animation::Damage });
      }
      // - player.attack.weak is failing a hit (hitting an invuln player)
      // - player.attack.strong is hitting a shield
      // - player.attack.sweep is played with player.hurt (and optionally
      //   player.attack.crit), when a player hits multiple targets
      // - player.attack.crit is played with player.hurt, when a player crits
      // - player.attack.knockback is similar to crit (idk when its used)
      // - player.hurt is the base hurt sound
      //
      // Note that the proxy will convert all of these to the 1.8 names
      p.send(cb::Packet::PlaySound {
        name: "entity.player.hurt".into(),
        category: cb::SoundCategory::Players,
        pos,
        volume: 1.0,
        pitch: 1.0,
      });
    }

    true
  }
}

impl CommandSender for Player {
  fn block_pos(&self) -> Option<Pos> { Some(self.block_pos()) }
}

#[test]
fn assert_sync() {
  fn is_sync<T: Send + Sync>() {}
  is_sync::<Player>(); // only compiles if player is Sync
}
