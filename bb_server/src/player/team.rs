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

#[derive(Debug)]
pub struct Team {
  name: String,

  /// Set of uuids for players on this team.
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
    let mut prefix = Chat::empty();
    prefix.add("[BIG] ").color(Color::Red);
    let t = Team {
      name: name.clone(),
      members: HashSet::new(),
      display_name: Chat::new(name.clone()),
      friendly_fire: true,
      see_invis: false,
      name_tag_rule: TeamRule::Always,
      collision_rule: TeamRule::Always,
      color: Color::White,
      prefix,
      postfix: Chat::new(" whaa"),
      wm,
    };
    let out = cb::Packet::Teams {
      team:   name,
      action: TeamAction::Create { info: t.info(), entities: vec![] },
    };
    t.wm.send_to_all(out);
    t
  }

  /// Returns the name of the team.
  pub fn name(&self) -> &String { &self.name }

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
      display_name:  self.display_name.clone(),
      friendly_fire: self.friendly_fire,
      see_invis:     self.see_invis,
      name_tag:      self.name_tag_rule,
      collisions:    self.collision_rule,
      color:         self.color.clone(),
      prefix:        self.prefix.clone(),
      postfix:       self.postfix.clone(),
    }
  }

  pub(crate) fn player_disconnect(&mut self, id: UUID) { self.members.remove(&id); }

  /// Creates the team for the player that has just joined.
  pub(crate) fn send_join(&self, player: &Player) {
    player.send(cb::Packet::Teams {
      team:   self.name.clone(),
      action: TeamAction::Create {
        info:     self.info(),
        entities: self
          .members
          .iter()
          .map(|id| self.wm.get_player(*id).unwrap().username().clone())
          .collect(),
      },
    });
  }
}
