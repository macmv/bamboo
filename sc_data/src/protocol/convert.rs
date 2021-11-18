pub fn class(name: &str) -> String {
  match name {
    // TODO: Generics
    "java/util/Map" => "HashMap<u8, u8>",
    "java/util/Set" => "HashSet<u8>",
    "java/util/Collection" => "HashMap<u8, u8>",
    "net/minecraft/util/registry/DynamicRegistryManager$Impl" => "u8",
    "net/minecraft/util/registry/RegistryKey" => "u8",
    "java/util/Optional" => "Option<u8>",
    "net/minecraft/util/Vec3" => "[u8; 3]",

    "java/util/List" => "Vec<u8>",
    "java/util/UUID" => "UUID",
    "java/lang/String" => "String",
    "java/util/BitSet" => "BitArray",
    "it/unimi/dsi/fastutil/ints/IntList" => "Vec<i32>",
    "it/unimi/dsi/fastutil/objects/Object2IntMap" => "HashMap<u8, i32>",
    "it/unimi/dsi/fastutil/ints/Int2ObjectMap" => "HashMap<i32, u8>",
    "net/minecraft/util/math/Vec3i" => "[i32; 3]",
    "net/minecraft/util/Vec4b" => "[bool; 4]",
    "net/minecraft/util/math/Vec4b" => "[bool; 4]",
    "net/minecraft/util/math/Vec3d" => "[f64; 3]",
    "net/minecraft/util/math/BlockPos" => "Pos",
    "net/minecraft/util/BlockPos" => "Pos",
    "net/minecraft/item/Item" => "u32",   // item id
    "net/minecraft/block/Block" => "u32", // block id
    "net/minecraft/entity/EntityType" => "u32",
    "net/minecraft/world/World" => "u32",
    "net/minecraft/world/Vibration" => "u8",
    "net/minecraft/util/EnumFacing" => "Face",
    "net/minecraft/util/math/Direction" => "Face",
    "net/minecraft/util/SoundCategory" => "u32",
    "net/minecraft/sound/SoundCategory" => "u32",
    "net/minecraft/item/crafting/IRecipe" => "u32",
    "net/minecraft/block/state/IBlockState" => "(u32, String)",
    "net/minecraft/block/BlockState" => "(u32, String)",
    "net/minecraft/text/Text" => "String",
    "net/minecraft/util/Identifier" => "String",
    "net/minecraft/util/Formatting" => "String",
    "net/minecraft/util/IChatComponent" => "String",
    "net/minecraft/util/ResourceLocation" => "String",
    "net/minecraft/util/text/ITextComponent" => "String",
    "net/minecraft/world/EnumDifficulty" => "u32",
    "net/minecraft/world/Difficulty" => "u32",
    "net/minecraft/item/ItemStack" => "Stack",
    "net/minecraft/inventory/EntityEquipmentSlot" => "Stack",
    "net/minecraft/entity/EquipmentSlot" => "Stack",
    "net/minecraft/network/play/server/SPacketUpdateBossInfo$Operation" => "BossType",
    "net/minecraft/network/packet/s2c/play/BossBarS2CPacket$Type" => "BossType",
    "net/minecraft/network/packet/s2c/play/BossBarS2CPacket$Action" => "BossAction",
    "net/minecraft/world/BossInfo$Color" => "BossColor",
    "net/minecraft/entity/boss/BossBar$Color" => "BossColor",
    "net/minecraft/world/BossInfo$Overlay" => "BossOverlay",
    "net/minecraft/entity/boss/BossBar$Style" => "BossStyle",
    "net/minecraft/network/packet/s2c/play/GameStateChangeS2CPacket$Reason" => "StateChangeReason",
    "net/minecraft/util/text/ChatType" => "ChatType",
    "net/minecraft/network/MessageType" => "MessageType",
    "net/minecraft/network/play/server/S21PacketChunkData$Extracted" => "ExtractedChunkData",
    "net/minecraft/nbt/CompoundTag" => "NBT",
    "net/minecraft/nbt/NbtCompound" => "NBT",
    "net/minecraft/nbt/NBTTagCompound" => "NBT",
    "net/minecraft/entity/DataWatcher"
    | "net/minecraft/network/datasync/EntityDataManager"
    | "net/minecraft/entity/data/DataTracker" => "EntityMetadata",
    "net/minecraft/world/biome/source/BiomeArray" => "Vec<u32>",
    "net/minecraft/network/play/server/SPacketCombatEvent$Event" => "CombatEvent",
    "net/minecraft/network/packet/s2c/play/CombatEventS2CPacket$Type" => "CombatEvent",
    "net/minecraft/network/play/server/S42PacketCombatEvent$Event" => "CombatEvent",
    "com/mojang/brigadier/suggestion/Suggestions" => "CommandSuggestions",
    "com/mojang/brigadier/tree/RootCommandNode" => "CommandNode",
    "net/minecraft/network/PacketBuffer" => "Vec<u8>",
    "net/minecraft/util/PacketByteBuf" => "Vec<u8>",
    "net/minecraft/network/PacketByteBuf" => "Vec<u8>",
    "net/minecraft/world/GameType" => "WorldType",
    "net/minecraft/world/WorldType" => "WorldType",
    "net/minecraft/world/WorldSettings$GameType" => "WorldType",
    "net/minecraft/world/GameMode" => "GameMode",
    "net/minecraft/world/dimension/DimensionType" => "WorldType",
    "net/minecraft/world/level/LevelGeneratorType" => "LevelType",
    "net/minecraft/command/arguments/EntityAnchorArgumentType$EntityAnchor" => "EntityAnchor",
    "net/minecraft/command/argument/EntityAnchorArgumentType$EntityAnchor" => "EntityAnchor",
    "net/minecraft/world/storage/MapDecoration" => "MapIcon",
    "net/minecraft/item/map/MapIcon" => "MapIcon",
    "net/minecraft/item/map/MapState$UpdateData" => "MapUpdate",
    "net/minecraft/util/math/ChunkPos" => "ChunkPos",
    "net/minecraft/world/ChunkCoordIntPair" => "ChunkPos",
    "net/minecraft/util/math/ChunkSectionPos" => "ChunkPos",
    "net/minecraft/network/play/server/S22PacketMultiBlockChange$BlockUpdateData"
    | "net/minecraft/network/play/server/SPacketMultiBlockChange$BlockUpdateData"
    | "net/minecraft/network/packet/s2c/play/ChunkDeltaUpdateS2CPacket$ChunkDeltaRecord" => {
      "MultiBlockChange"
    }
    "net/minecraft/util/Hand"
    | "net/minecraft/util/EnumHand"
    | "net/minecraft/util/EnumHandSide"
    | "net/minecraft/util/Arm" => "Hand",
    "net/minecraft/util/EnumParticleTypes" => "u32",
    "net/minecraft/particle/ParticleEffect" => "u32",
    "net/minecraft/util/SoundEvent" => "u32",
    "net/minecraft/sound/SoundEvent" => "u32",
    "org/apache/logging/log4j/Logger" => "Log",
    "net/minecraft/network/packet/c2s/play/PlayerActionC2SPacket$Action" => "PlayerAction",
    "net/minecraft/network/play/server/S38PacketPlayerListItem$Action" => "PlayerListAction",
    "net/minecraft/network/play/server/SPacketPlayerListItem$Action" => "PlayerListAction",
    "net/minecraft/network/packet/s2c/play/PlayerListS2CPacket$Action" => "PlayerListAction",
    "net/minecraft/network/play/server/SPacketRecipeBook$State" => "RecipeBookState",
    "net/minecraft/potion/Potion" => "u32",
    "net/minecraft/entity/effect/StatusEffect" => "u32",
    "net/minecraft/scoreboard/IScoreObjectiveCriteria$EnumRenderType"
    | "net/minecraft/scoreboard/IScoreCriteria$EnumRenderType"
    | "net/minecraft/scoreboard/ScoreboardCriterion$RenderType" => "ScoreboardDisplayType",
    "net/minecraft/village/TraderOfferList" | "net/minecraft/village/TradeOfferList" => "TradeList",
    "net/minecraft/tag/RegistryTagManager" => "TagManager",
    "net/minecraft/tag/TagManager" => "TagManager",
    "net/minecraft/network/play/server/S45PacketTitle$Type"
    | "net/minecraft/network/play/server/SPacketTitle$Type"
    | "net/minecraft/network/packet/s2c/play/TitleS2CPacket$Action" => "TitleType",
    "net/minecraft/network/packet/s2c/play/UnlockRecipesS2CPacket$Action" => "UnlockRecipeType",
    "net/minecraft/recipe/book/RecipeBookOptions" => "RecipeBookOption",
    "net/minecraft/network/play/server/S3CPacketUpdateScore$Action"
    | "net/minecraft/network/play/server/SPacketUpdateScore$Action"
    | "net/minecraft/scoreboard/ServerScoreboard$UpdateMode" => "ScoreboardUpdateType",
    "net/minecraft/network/play/server/S44PacketWorldBorder$Action"
    | "net/minecraft/network/play/server/SPacketWorldBorder$Action"
    | "net/minecraft/network/packet/s2c/play/WorldBorderS2CPacket$Type" => "WorldBorderType",
    "net/minecraft/network/packet/c2s/play/AdvancementTabC2SPacket$Action" => "AdvancementTabType",
    "net/minecraft/screen/slot/SlotActionType" | "net/minecraft/container/SlotActionType" => {
      "SlotAction"
    }
    "net/minecraft/inventory/ClickType" => "ClickType",
    "net/minecraft/network/packet/c2s/play/ClientCommandC2SPacket$Mode" => "CommandMode",
    "net/minecraft/entity/player/EntityPlayer$EnumChatVisibility"
    | "net/minecraft/client/options/ChatVisibility"
    | "net/minecraft/client/option/ChatVisibility" => "ChatVisibility",
    "net/minecraft/network/play/client/C16PacketClientStatus$EnumState"
    | "net/minecraft/network/play/client/CPacketClientStatus$State"
    | "net/minecraft/network/packet/c2s/play/ClientStatusC2SPacket$Mode" => "ClientState",
    "net/minecraft/network/play/client/C0BPacketEntityAction$Action"
    | "net/minecraft/network/play/client/CPacketEntityAction$Action" => "EntityAction",
    "net/minecraft/network/play/client/C07PacketPlayerDigging$Action"
    | "net/minecraft/network/play/client/CPacketPlayerDigging$Action" => "DiggingAction",
    "net/minecraft/util/hit/BlockHitResult" => "BlockHit",
    "net/minecraft/network/packet/c2s/play/RecipeBookDataC2SPacket$Mode" => "RecipeBookMode",
    "net/minecraft/recipe/book/RecipeBookCategory" => "RecipeBookCategory",
    "net/minecraft/network/play/client/CPacketRecipeInfo$Purpose" => "RecipePurpose",
    "net/minecraft/network/play/client/C19PacketResourcePackStatus$Action"
    | "net/minecraft/network/play/client/CPacketResourcePackStatus$Action"
    | "net/minecraft/network/packet/c2s/play/ResourcePackStatusC2SPacket$Status" => {
      "ResourcePacketAction"
    }
    "net/minecraft/network/play/client/CPacketSeenAdvancements$Action" => "SeenAdvancements",
    "net/minecraft/block/entity/CommandBlockBlockEntity$Type" => "CommandBlockType",
    "net/minecraft/block/entity/JigsawBlockEntity$Joint" => "JigsawBlockType",
    "net/minecraft/block/entity/StructureBlockBlockEntity$Action" => "StructureBlockType",
    "net/minecraft/block/enums/StructureBlockMode" => "StructureBlockMode",
    "net/minecraft/util/BlockMirror" => "u32",
    "net/minecraft/util/BlockRotation" => "Face",
    "net/minecraft/network/play/client/C02PacketUseEntity$Action"
    | "net/minecraft/network/play/client/CPacketUseEntity$Action" => "UseEntity",

    _ => {
      println!("unknown type {}", name);
      name.split('/').last().unwrap().split('$').next().unwrap()
    }
  }
  .into()
}

pub fn static_call(name: &str) -> &str {
  match name {
    "newHashMap" => "HashMap::new",
    "newLinkedHashSet" => "HashSet::new",
    _ => {
      println!("unknown static call {}", name);
      name
    }
  }
}

pub fn member_call(name: &str) -> &str {
  match name {
    "add" => "insert",
    "put" => "insert",
    "read_byte" => "read_u8",
    // Booleans are always converted with `!= 0`, so it is best to read them as bytes.
    "read_boolean" => "read_u8",
    "read_int" => "read_i32",
    "read_long" => "read_i64",
    "read_float" => "read_f32",
    "read_double" => "read_f64",
    _ => {
      println!("unknown member call {}", name);
      name
    }
  }
}
