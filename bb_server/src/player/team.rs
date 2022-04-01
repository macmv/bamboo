use super::Player;
use crate::world::WorldManager;
use bb_common::{
  net::{
    cb,
    cb::{TeamAction, TeamInfo, TeamRule},
  },
  util::{chat::Color, Chat, UUID},
};
use std::{collections::HashSet, sync::Arc};

pub struct Team {
  name: String,

  // Set of player ids
  members: HashSet<UUID>,

  display_name:   Chat,
  friendly_fire:  bool,
  see_invis:      bool,
  name_tag_rule:  TeamRule,
  collision_rule: TeamRule,
  color:          Color,
  prefix:         Chat,
  postfix:        Chat,

  wm: Arc<WorldManager>,
}

impl Team {
  pub fn new(wm: Arc<WorldManager>, name: String) -> Self {
    let t = Team {
      name: name.clone(),
      members: HashSet::new(),
      display_name: Chat::new(name.clone()),
      friendly_fire: true,
      see_invis: false,
      name_tag_rule: TeamRule::Always,
      collision_rule: TeamRule::Always,
      color: Color::White,
      prefix: Chat::empty(),
      postfix: Chat::empty(),
      wm,
    };
    let out = cb::Packet::Teams {
      team:   name,
      action: TeamAction::Create { info: t.info(), entities: vec![] },
    };
    t.wm.send_to_all(out);
    t
  }

  pub fn set_prefix(&mut self, prefix: Chat) {
    self.prefix = prefix;
    self.update_info();
  }
  pub fn set_postfix(&mut self, postfix: Chat) {
    self.postfix = postfix;
    self.update_info();
  }
  pub fn set_color(&mut self, color: Color) {
    self.color = color;
    self.update_info();
  }

  pub fn add(&mut self, player: &Player) {
    self.members.insert(player.id());
    let out = cb::Packet::Teams {
      team:   self.name.clone(),
      action: TeamAction::AddEntities { entities: vec![player.username().clone()] },
    };
    self.wm.send_to_all(out);
  }

  fn update_info(&self) {
    let out = cb::Packet::Teams {
      team:   self.name.clone(),
      action: TeamAction::UpdateInfo { info: self.info() },
    };
    self.wm.send_to_all(out);
  }
  fn info(&self) -> TeamInfo {
    TeamInfo {
      display_name:  self.display_name.to_json(),
      friendly_fire: self.friendly_fire,
      see_invis:     self.see_invis,
      name_tag:      self.name_tag_rule,
      collisions:    self.collision_rule,
      color:         self.color.clone(),
      prefix:        self.prefix.to_json(),
      postfix:       self.prefix.to_json(),
    }
  }
}
