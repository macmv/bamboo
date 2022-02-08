use super::{Command, CommandTree, NodeType, Parser, StringType};
use sc_common::{
  net::{
    cb,
    cb::{CommandNode, CommandType},
  },
  util::Buffer,
};

impl CommandTree {
  /// Serializes the entire command tree. This will be called any time a player
  /// joins.
  pub fn serialize(&self) -> cb::Packet {
    // This is a reverse-order list of all the nodes. The highest level node (the
    // root node) will be last.
    let mut nodes = vec![];

    let commands = self.commands.lock();
    let c = Command {
      name:     "".into(),
      ty:       NodeType::Root,
      children: commands.values().map(|(command, _)| command.clone()).collect(),
    };
    c.write_nodes(&mut nodes);

    cb::Packet::CommandList { root: nodes.len() as u32 - 1, nodes }
  }
}

impl Command {
  // Adds all children in order from the lowest nodes up. All dependencies must
  // already be in the list before a node can be written.
  //
  // Returns the index of self into the array.
  fn write_nodes(&self, nodes: &mut Vec<CommandNode>) -> u32 {
    let children = self.children.iter().map(|c| c.write_nodes(nodes)).collect();
    nodes.push(CommandNode {
      ty: self.ty.as_ty(),
      executable: false,
      children,
      redirect: None,
      name: self.name.clone(),
      parser: match &self.ty {
        NodeType::Argument(parser) => parser.name().into(),
        _ => "".into(),
      },
      properties: match &self.ty {
        NodeType::Argument(parser) => {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          parser.write_data(&mut buf);
          data
        }
        _ => vec![],
      },
      suggestion: None,
    });
    (nodes.len() - 1) as u32
  }
}

impl NodeType {
  fn as_ty(&self) -> CommandType {
    match self {
      Self::Root => CommandType::Root,
      Self::Literal => CommandType::Literal,
      Self::Argument(_) => CommandType::Argument,
    }
  }
}

impl Parser {
  /// Returns the name of this parser. Used in packet serialization.
  #[rustfmt::skip]
  pub fn name(&self) -> &'static str {
    match self {
      Self::Bool               => "brigadier:bool",
      Self::Double { .. }      => "brigadier:double",
      Self::Float { .. }       => "brigadier:float",
      Self::Int { .. }         => "brigadier:int",
      Self::String(_)          => "brigadier:string",
      Self::Entity { .. }      => "minecraft:entity",
      Self::ScoreHolder { .. } => "minecraft:score_holder",
      Self::GameProfile        => "minecraft:game_profile",
      Self::BlockPos           => "minecraft:block_pos",
      Self::ColumnPos          => "minecraft:column_pos",
      Self::Vec3               => "minecraft:vec3",
      Self::Vec2               => "minecraft:vec2",
      Self::BlockState         => "minecraft:block_state",
      Self::BlockPredicate     => "minecraft:block_predicate",
      Self::ItemStack          => "minecraft:item_stack",
      Self::ItemPredicate      => "minecraft:item_predicate",
      Self::Color              => "minecraft:color",
      Self::Component          => "minecraft:component",
      Self::Message            => "minecraft:message",
      Self::Nbt                => "minecraft:nbt",
      Self::NbtPath            => "minecraft:nbt_path",
      Self::Objective          => "minecraft:objective",
      Self::ObjectiveCriteria  => "minecraft:objective_criteria",
      Self::Operation          => "minecraft:operation",
      Self::Particle           => "minecraft:particle",
      Self::Rotation           => "minecraft:rotation",
      Self::Angle              => "minecraft:angle",
      Self::ScoreboardSlot     => "minecraft:scoreboard_slot",
      Self::Swizzle            => "minecraft:swizzle",
      Self::Team               => "minecraft:team",
      Self::ItemSlot           => "minecraft:item_slot",
      Self::ResourceLocation   => "minecraft:resource_location",
      Self::MobEffect          => "minecraft:mob_effect",
      Self::Function           => "minecraft:function",
      Self::EntityAnchor       => "minecraft:entity_anchor",
      Self::Range { .. }       => "minecraft:range",
      Self::IntRange           => "minecraft:int_range",
      Self::FloatRange         => "minecraft:float_range",
      Self::ItemEnchantment    => "minecraft:item_enchantment",
      Self::EntitySummon       => "minecraft:entity_summon",
      Self::Dimension          => "minecraft:dimension",
      Self::Uuid               => "minecraft:uuid",
      Self::NbtTag             => "minecraft:nbt_tag",
      Self::NbtCompoundTag     => "minecraft:nbt_compound_tag",
      Self::Time               => "minecraft:time",
      Self::Modid              => "forge:modid",
      Self::Enum               => "forge:enum",
    }
  }

  /// If this parser stores any extra data, that will be written to the buffer.
  /// Most nodes will not write any extra data.
  pub fn write_data<T>(&self, buf: &mut Buffer<T>)
  where
    std::io::Cursor<T>: std::io::Write,
  {
    match self {
      Self::Double { min, max } => {
        let mut bitmask = 0;
        if min.is_some() {
          bitmask |= 0x01;
        }
        if max.is_some() {
          bitmask |= 0x02;
        }
        buf.write_u8(bitmask);
        if let Some(min) = min {
          buf.write_f64(*min);
        }
        if let Some(max) = max {
          buf.write_f64(*max);
        }
      }
      Self::Float { min, max } => {
        let mut bitmask = 0;
        if min.is_some() {
          bitmask |= 0x01;
        }
        if max.is_some() {
          bitmask |= 0x02;
        }
        buf.write_u8(bitmask);
        if let Some(min) = min {
          buf.write_f32(*min);
        }
        if let Some(max) = max {
          buf.write_f32(*max);
        }
      }
      Self::Int { min, max } => {
        let mut bitmask = 0;
        if min.is_some() {
          bitmask |= 0x01;
        }
        if max.is_some() {
          bitmask |= 0x02;
        }
        buf.write_u8(bitmask);
        if let Some(min) = min {
          buf.write_i32(*min);
        }
        if let Some(max) = max {
          buf.write_i32(*max);
        }
      }
      Self::String(ty) => {
        buf.write_varint(match ty {
          StringType::Word => 0,
          StringType::Quotable => 1,
          StringType::Greedy => 2,
        });
      }
      Self::Entity { single, players } => {
        let mut bitmask = 0;
        if *single {
          bitmask |= 0x01;
        }
        if *players {
          bitmask |= 0x02;
        }
        buf.write_u8(bitmask);
      }
      Self::ScoreHolder { multiple } => {
        let mut bitmask = 0;
        if *multiple {
          bitmask |= 0x01;
        }
        buf.write_u8(bitmask);
      }
      Self::Range { decimals } => {
        buf.write_bool(*decimals);
      }
      _ => {}
    }
  }
}
