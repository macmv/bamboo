use num_derive::{FromPrimitive, ToPrimitive};

use crate::proto;

#[derive(Clone, Debug)]
pub struct Packet {
  id: ID,
  pb: proto::Packet,
}

impl Packet {
  pub fn new(id: ID) -> Self {
    Packet { id, pb: proto::Packet::default() }
  }
  pub fn from_proto(pb: proto::Packet) -> Self {
    let id = ID::from_i32(pb.id);
    Packet { id, pb }
  }
  pub fn id(&self) -> ID {
    self.id
  }
  pub fn pb(&self) -> &proto::Packet {
    &self.pb
  }
}

impl ID {
  pub fn to_i32(&self) -> i32 {
    num::ToPrimitive::to_i32(self).unwrap()
  }
  pub fn from_i32(id: i32) -> Self {
    num::FromPrimitive::from_i32(id).unwrap()
  }
}

#[derive(Clone, Copy, FromPrimitive, ToPrimitive, Debug, PartialEq, Eq, Hash)]
pub enum ID {
  SpawnEntity = 0,
  SpawnExpOrb,
  SpawnWeatherEntity,
  SpawnLivingEntity,
  SpawnPainting,
  SpawnPlayer,
  EntityAnimation,
  Statistics,
  AcknowledgePlayerDigging,
  BlockBreakAnimation,
  BlockEntityData,
  BlockAction,
  BlockChange,
  BossBar,
  ServerDifficulty,
  ChatMessage,
  MultiBlockChange,
  TabComplete,
  DeclareCommands,
  WindowConfirm,
  CloseWindow,
  WindowItems,
  WindowProperty,
  SetSlot,
  SetCooldown,
  PluginMessage,
  NamedSoundEffect,
  Disconnect,
  EntityStatus,
  Explosion,
  UnloadChunk,
  ChangeGameState,
  OpenHorseWindow,
  KeepAlive,
  ChunkData,
  Effect,
  Particle,
  UpdateLight,
  JoinGame,
  MapData,
  TradeList,
  EntityPosition,
  EntityPositionAndRotation,
  EntityRotation,
  EntityOnGround,
  VehicleMove,
  OpenBook,
  OpenWindow,
  OpenSignEditor,
  CraftRecipeResponse,
  PlayerAbilities,
  EnterCombat,
  PlayerInfo,
  FacePlayer,
  PlayerPositionAndLook,
  UnlockRecipies,
  DestroyEntity,
  RemoveEntityEffect,
  ResourcePack,
  Respawn,
  EntityHeadLook,
  SelectAdvancementTab,
  WorldBorder,
  Camera,
  HeldItemChange,
  UpdateViewPosition,
  UpdateViewDistance,
  DisplayScoreboard,
  EntityMetadata,
  AttachEntity,
  EntityVelocity,
  EntityEquipment,
  SetExp,
  UpdateHealth,
  ScoreboardObjective,
  SetPassengers,
  Teams,
  UpdateScore,
  SpawnPosition,
  TimeUpdate,
  Title,
  EntitySoundEffect,
  SoundEffect,
  StopSound,
  PlayerListHeader,
  NBTQueryResponse,
  CollectItem,
  EntityTeleport,
  Advancements,
  EntityProperties,
  EntityEffect,
  DeclareRecipies,
  Tags,
  // Custom packet; should be intercepted by the proxy,
  Login,
  None,
}
