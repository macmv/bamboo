use super::{Expr, Instr, Value};

pub fn class(field: &str, name: &str) -> String {
  match name.split('/').last().unwrap() {
    "Map" => "HashMap<U, U>",
    "Set" => "HashSet<U>",
    "Collection" => "HashMap<U, U>",
    "DynamicRegistryManager$Impl" => "U",
    "RegistryKey" => "U",
    "Vec3" => "[U; 3]",
    "Optional" => "Option<String>",

    "List" => match field {
      "pages" => "Vec<String>",
      "recipe_ids_to_init" => "Vec<String>",
      "recipe_ids_to_change" => "Vec<String>",
      "field_189557_e" => "Vec<U>",
      "tile_entity_tags" => "Vec<NBT>",
      _ => {
        println!("UNKNOWN FIELD {}", field);
        "Vec<U>"
      }
    },
    "UUID" => "UUID",
    "String" => "String",
    "BitSet" => "BitSet",
    "IntList" => "Vec<i32>",
    "Object2IntMap" => "HashMap<U, i32>",
    "Int2ObjectMap" => "HashMap<i32, U>",
    "Vec3i" => "[i32; 3]",
    "Vec4b" => "[bool; 4]",
    "Vec3d" => "[f64; 3]",
    "BlockPos" => "Pos",
    "Item" => "u32",  // item id
    "Block" => "u32", // block id
    "EntityType" => "u32",
    "Vibration" => "U",
    "IBlockState" | "BlockState" => "(u32, String)",
    "Formatting" => "i32",
    "Text" | "Identifier" | "IChatComponent" | "ResourceLocation" | "ITextComponent" => "String",
    "Difficulty" | "EnumDifficulty" => "u32",
    "ItemStack" => "Stack",
    "GameStateChangeS2CPacket$Reason" => "StateChangeReason",
    "S21PacketChunkData$Extracted" => "Vec<u8>",
    "CompoundTag" | "NbtCompound" | "NBTTagCompound" => "NBT",
    "DataWatcher" | "EntityDataManager" | "DataTracker" => "EntityMetadata",
    "BiomeArray" => "Vec<u32>",
    "Suggestions" => "CommandSuggestions",
    "RootCommandNode" => "CommandNode",
    "PacketBuffer" | "PacketByteBuf" => "Vec<u8>",
    "GameType" | "WorldSettings$GameType" => "NBT",
    "GameMode" => "GameMode",
    "DimensionType" => "NBT",
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

pub fn static_call<'a, 'b>(class: &'a str, name: &'b str) -> (&'a str, &'b str) {
  match (class, name) {
    ("net/minecraft/network/PacketByteBuf", _) => (
      "tcp::Packet",
      match name {
        "read_var_int" => "read_varint",
        "read_item_stack" => "read_item",
        "read_identifier" => "read_ident",
        "read_nbt" => "read_nbt",
        "read_string" => "read_str",
        "get_max_validator" => "get_max_validator", // Parsed out later
        _ => panic!("unknown packet function {}", name),
      },
    ),
    (_, "new_hash_map") => ("HashMap", "new"),
    (_, "new_linked_hash_set") | (_, "new_hash_set") => ("HashSet", "new"),
    (_, "new_array_list") => ("Vec", "new"),
    _ => {
      println!("unknown static call {}::{}", class, name);
      (class, name)
    }
  }
}

pub fn member_call<'a>(class: &str, name: &'a str) -> (&'a str, Option<Vec<Expr>>) {
  (
    match name {
      "add" => match class {
        "java/util/List" => "push",
        "java/util/Deque" => "push",
        "java/util/Collection" => "insert",
        "java/util/Set" => "insert",
        _ => panic!("unknown class for add {}", class),
      },
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
      "read_bytes" => "read_buf",           // Fixed length
      "read_byte_array" => "read_byte_arr", // Variable length
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
      "read_optional" => "read_option",
      "read_map" => "read_map",
      "read_list" => "read_list",
      "read_nbt" => "read_nbt",
      _ => {
        println!("unknown member call {}", name);
        name
      }
    },
    None,
  )
}

pub fn reader_func_to_ty(field: &str, name: &str) -> &'static str {
  match name {
    "read_boolean" => "bool",
    "read_varint" => "i32",
    "read_u8" => "u8",
    "read_i8" => "i8",
    "read_i16" => "i16",
    "read_i32" => "i32",
    "read_option" => "Option<String>", // Literally used once in the entire 1.17 codebase.
    "read_i64" => "i64",
    "read_f32" => "f32",
    "read_f64" => "f64",
    "read_pos" => "Pos",
    "read_item" => "Stack",
    "read_uuid" => "UUID",
    "read_str" => "String",
    "read_nbt" => "NBT",
    "read_buf" | "read_byte_arr" => "Vec<u8>",
    "read_i32_arr" => "Vec<i32>",
    "read_varint_arr" => "Vec<i32>",
    "read_bits" => "BitSet",
    "read_block_hit" => "BlockHit",

    "read_map" => "u8",
    "read_list" => match field {
      "recipe_ids_to_init" => "Vec<String>",
      "recipe_ids_to_change" => "Vec<String>",
      _ => "Vec<u8>",
    },
    "read_collection" => match field {
      "pages" => "Vec<String>",
      _ => "Vec<u8>",
    },
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
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "U" => match from {
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
    "HashMap<U, U>" | "HashMap<U, i32>" | "HashSet<U>" | "Vec<U>" => return "",
    "String" => match from {
      _ => return "",
    },
    _ => panic!("cannot convert `{}` into `{}`", from, to),
  }
}

pub fn this_call(name: &str, args: &mut Vec<Expr>) -> Instr {
  assert_eq!(args.len(), 1);
  Instr::Set(
    match name {
      "setInvulnerable" => "invulnerable",
      "setFlying" => "flying",
      "setAllowFlying" => "allow_flying",
      "setCreativeMode" => "creative_mode",
      "setFlySpeed" => "fly_speed",
      "setWalkSpeed" => "walk_speed",
      "setFovModifier" => "fov_modifier",
      _ => panic!("unknown `this` call: {}", name),
    }
    .into(),
    args.pop().unwrap(),
  )
}
