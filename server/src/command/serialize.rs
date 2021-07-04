use super::{Command, CommandTree, NodeType};
use common::{net::cb, util::Buffer};

impl NodeType {
  fn mask(&self) -> u8 {
    match self {
      Self::Root => 0x00,
      Self::Literal => 0x01,
      Self::Argument(_) => 0x02,
    }
  }
}

struct IndexNode {
  name:     String,
  ty:       NodeType,
  children: Vec<usize>,
}

impl CommandTree {
  /// Serializes the entire command tree. This will be called any time a player
  /// joins.
  pub fn serialize(&self) -> cb::Packet {
    // This is a reverse-order list of all the nodes. The highest level node (the
    // root node) will be last.
    let mut nodes = vec![];

    let commands = self.commands.lock().unwrap();
    let c = Command {
      name:     "".into(),
      ty:       NodeType::Root,
      children: commands.values().cloned().collect(),
    };
    c.write_nodes(&mut nodes);

    let mut data = Buffer::new(vec![]);

    for node in &nodes {
      let mask = node.ty.mask();
      // TODO: Check executable bits
      data.write_u8(mask);
      data.write_varint(node.children.len() as i32);
      for &index in &node.children {
        data.write_varint(index as i32);
      }
      match &node.ty {
        NodeType::Argument(parser) => {
          data.write_str(&node.name);
          data.write_str(&parser.name);
          parser.write_data(&mut data);
        }
        NodeType::Literal => {
          data.write_str(&node.name);
        }
        NodeType::Root => {}
      }
    }

    let mut out = cb::Packet::new(cb::ID::DeclareCommands);
    out.set_int("root", (nodes.len() - 1) as i32); // The root index is always last
    out.set_byte_arr("data", data.into_inner());
    out
  }
}

impl Command {
  // Adds all children in order from the lowest nodes up. All dependencies must
  // already be in the list before a node can be written.
  //
  // Returns the index of self into the array.
  fn write_nodes(&self, nodes: &mut Vec<IndexNode>) -> usize {
    let children = self.children.iter().map(|c| c.write_nodes(nodes)).collect();
    nodes.push(IndexNode { name: self.name.clone(), ty: self.ty.clone(), children });
    nodes.len() - 1
  }
}
