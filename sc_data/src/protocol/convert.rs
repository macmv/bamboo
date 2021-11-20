use super::{Expr, Value};

pub fn class(name: &str) -> String {
  match name.split('/').last().unwrap() {
    // TODO: Generics
    "Map" => "HashMap<u8, u8>",
    "Set" => "HashSet<u8>",
    "Collection" => "HashMap<u8, u8>",
    "DynamicRegistryManager$Impl" => "u8",
    "RegistryKey" => "u8",
    "Optional" => "Option<u8>",
    "Vec3" => "[u8; 3]",

    "List" => "Vec<u8>",
    "UUID" => "UUID",
    "String" => "String",
    "BitSet" => "BitSet",
    "IntList" => "Vec<i32>",
    "Object2IntMap" => "HashMap<u8, i32>",
    "Int2ObjectMap" => "HashMap<i32, u8>",
    "Vec3i" => "[i32; 3]",
    "Vec4b" => "[bool; 4]",
    "Vec3d" => "[f64; 3]",
    "BlockPos" => "Pos",
    "Item" => "u32",  // item id
    "Block" => "u32", // block id
    "EntityType" => "u32",
    "Vibration" => "u8",
    "IBlockState" | "BlockState" => "(u32, String)",
    "Text" | "Identifier" | "Formatting" | "IChatComponent" | "ResourceLocation"
    | "ITextComponent" => "String",
    "Difficulty" | "EnumDifficulty" => "u32",
    "ItemStack" => "Stack",
    "GameStateChangeS2CPacket$Reason" => "StateChangeReason",
    "S21PacketChunkData$Extracted" => "ExtractedChunkData",
    "CompoundTag" | "NbtCompound" | "NBTTagCompound" => "NBT",
    "DataWatcher" | "EntityDataManager" | "DataTracker" => "EntityMetadata",
    "BiomeArray" => "Vec<u32>",
    "Suggestions" => "CommandSuggestions",
    "RootCommandNode" => "CommandNode",
    "PacketBuffer" | "PacketByteBuf" => "Vec<u8>",
    "GameType" | "WorldSettings$GameType" => "WorldType",
    "GameMode" => "GameMode",
    "DimensionType" => "WorldType",
    "LevelGeneratorType" => "LevelType",
    "MapDecoration" => "MapIcon",
    "MapState$UpdateData" => "MapUpdate",
    "ChunkCoordIntPair" | "ChunkSectionPos" => "ChunkPos",
    "S22PacketMultiBlockChange$BlockUpdateData"
    | "SPacketMultiBlockChange$BlockUpdateData"
    | "ChunkDeltaUpdateS2CPacket$ChunkDeltaRecord" => "MultiBlockChange",
    "EnumParticleTypes" => "u32",
    "ParticleEffect" => "u32",
    "SoundEvent" => "u32",
    "Logger" => "Log",
    "TradeOfferList" => "TradeList",
    "RegistryTagManager" => "TagManager",
    "TagManager" => "TagManager",
    "RecipeBookOptions" => "RecipeBookOption",
    "BlockHitResult" => "BlockHit",
    "JigsawBlockEntity$Joint" => "JigsawBlockType",

    _ => "i32",
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

pub fn member_call(name: &str) -> (&str, Option<Vec<Expr>>) {
  (
    match name {
      "add" => "insert",
      "put" => "insert",
      "read_var_int" | "read_var_int_from_buffer" => "read_varint",
      // TODO: Might want to implement varlongs.
      "read_var_long" => "read_varint",
      // Booleans are always converted with `!= 0`, so it is best to read them as bytes.
      "read_boolean" => "read_u8",
      "read_unsigned_byte" => "read_u8",
      "read_byte" => "read_u8",
      "read_short" => "read_i16",
      "read_int" => "read_i32",
      "read_long" => "read_i64",
      "read_float" => "read_f32",
      "read_double" => "read_f64",
      "read_string" | "read_string_from_buffer" => "read_str",
      "read_var_int_array" => "read_varint_arr",
      "read_int_array" | "read_int_list" => "read_i32_arr",
      "read_byte_array" => "read_buf",
      "read_enum_constant" | "read_enum_value" => return ("read_varint", Some(vec![])),
      "read_text_component"
      | "read_text"
      | "read_identifier"
      | "read_chat_component"
      | "func_192575_l" => return ("read_str", Some(vec![Expr::new(Value::Lit(32767.into()))])),
      "read_item_stack_from_buffer" | "read_item_stack" => "read_item",
      "read_nbt_tag_compound_from_buffer" | "read_compound_tag" => "read_nbt",
      // This is the `read_block_hit` function in 1.17:
      // ```java
      // BlockPos pos = this.readBlockPos();
      // Direction dir = (Direction)this.readEnumConstant(Direction.class);
      // float x = this.readFloat();
      // float y = this.readFloat();
      // float z = this.readFloat();
      // boolean bl = this.readBoolean();
      // return new BlockHitResult(
      //   new Vec3d(
      //     (double)pos.getX() + (double)x,
      //     (double)pos.getY() + (double)y,
      //     (double)pos.getZ() + (double)z,
      //   ),
      //   dir,
      //   pos,
      //   bl,
      // );
      // ```
      "read_block_hit_result" => "read_block_hit",
      "read_block_pos" => "read_pos",

      // TODO: What is this??
      "decode" => "read_u8",
      _ => {
        println!("unknown member call {}", name);
        name
      }
    },
    None,
  )
}
