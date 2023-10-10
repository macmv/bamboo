use bb_common::util::Chat;

use crate::command::{Command, CommandTree, Parser, StringType};

pub struct SummonCommand {}

impl SummonCommand {
  pub fn init(commands: &CommandTree) {
    let mut c = Command::new("summon");
    c.add_arg("entity", Parser::EntitySummon);
    commands.add(c, |_, player, args| {
      // args[0] is `summon`
      let ty = args[1].entity_summon();
      if let Some(p) = player {
        let eid = p.world().summon(ty, p.pos());
        info!("eid of mob: {}", eid);
        p.send_message(Chat::new(format!("summoned {ty:?}")));
      }
    });
  }
}
