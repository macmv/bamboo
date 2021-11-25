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
    "new_hash_map" => "HashMap::new",
    "new_linked_hash_set" | "new_hash_set" => "HashSet::new",
    "new_array_list" => "Vec::new",
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
      "read_byte" => "read_i8",
      "read_short" => "read_i16",
      "read_int" => "read_i32",
      "read_long" => "read_i64",
      "read_float" => "read_f32",
      "read_double" => "read_f64",
      "read_uuid" => "read_uuid",
      "read_string" | "read_string_from_buffer" => "read_str",
      "read_var_int_array" => "read_varint_arr",
      "read_int_array" | "read_int_list" => "read_i32_arr",
      "read_byte_array" | "read_bytes" => "read_buf",
      "read_bit_set" => "read_bits",
      "read_enum_constant" | "read_enum_value" => return ("read_varint", Some(vec![])),
      "read_text_component"
      | "read_text"
      | "read_identifier"
      | "read_chat_component"
      | "func_192575_l" => return ("read_str", Some(vec![Expr::new(Value::Lit(32767.into()))])),
      "read_item_stack_from_buffer" | "read_item_stack" => "read_item",
      "read_nbt_tag_compound_from_buffer" | "read_compound_tag" => "read_nbt",
      "decode" => return ("read_nbt", Some(vec![])),
      "read_block_hit_result" => "read_block_hit",
      "read_block_pos" => "read_pos",
      "readable_bytes" => "remaining",
      _ => {
        println!("unknown member call {}", name);
        name
      }
    },
    None,
  )
}

pub fn reader_func_to_ty(name: &str) -> &str {
  match name {
    "read_boolean" => "bool",
    "read_varint" => "i32",
    "read_u8" => "u8",
    "read_i8" => "i8",
    "read_i16" => "i16",
    "read_i32" => "i32",
    "read_optional" => "i32", // Literally used once in the entire 1.17 codebase.
    "read_i64" => "i64",
    "read_f32" => "f32",
    "read_f64" => "f64",
    "read_pos" => "Pos",
    "read_item" => "Stack",
    "read_uuid" => "UUID",
    "read_str" => "String",
    "read_nbt" => "NBT",
    "read_buf" => "Vec<u8>",
    "read_i32_arr" => "Vec<i32>",
    "read_varint_arr" => "Vec<i32>",
    "read_bits" => "BitSet",
    "read_block_hit" => "BlockHit",

    "read_map" => "u8",
    "read_list" => "u8",
    "read_collection" => "u8",
    _ => panic!("unknown reader function {}", name),
  }
}

pub fn ty(from: &str, to: &str) -> &'static str {
  match to {
    "bool" => " != 0",
    "f32" => " as f32",
    "f64" => " as f64",
    "u8" => match from {
      "i8" | "i16" | "i32" | "i64" => ".try_into().unwrap()",
      "NBT" => "",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "i16" => match from {
      "u8" | "i8" => ".into()",
      "i32" | "i64" => ".try_into().unwrap()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "i32" => match from {
      "f32" => " as i32",
      "u8" | "i8" | "i16" => ".into()",
      "i64" => ".try_into().unwrap()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "i64" => match from {
      "f32" => " as i64",
      "u8" | "i8" | "i16" | "i32" => ".into()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "HashMap<u8, u8>" | "HashMap<u8, i32>" | "HashSet<u8>" | "Vec<u8>" => return "",
    "String" => match from {
      _ => return "",
    },
    "Option<u8>" => match from {
      "i32" => ".unwrap_or(0).into()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    _ => panic!("cannot convert `{}` into `{}`", from, to),
  }
}
