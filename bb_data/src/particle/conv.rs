fn old_to_new(old: &str) -> Option<String> {
  match old {
    "explode" => "poof",                          // very small explosion
    "largeexplode" => "explosion",                // normal explosion
    "hugeexplosion" => "explosion_emitter",       // large explosion
    "fireworksSpark" => "firework",               // firework trail
    "bubble" => "bubble",                         // light blue bubbles
    "splash" => "splash",                         // dark blue splashes
    "wake" => "fishing",                          // fishing bobber
    "suspended" => return None,                   // I cannot tell (the particle is too short)
    "depthsuspend" => "mycelium",                 // small gray particles that kinda hover
    "crit" => "crit",                             // shows when you crit an entity
    "magicCrit" => "enchanted_hit",               // blue particles shown when attacking armor
    "smoke" => "smoke",                           // black dust particles, which spread out
    "largesmoke" => "large_smoke",                // larger version of smoke
    "spell" => "effect",                          // spirals, used when you have an effect
    "instantSpell" => "instant_effect",           // crosses moving upward, used on splash potion
    "mobSpell" => "entity_effect",                // black spirals, seems the same as effect
    "mobSpellAmbient" => "ambient_entity_effect", // gray spirals, seems the same as effect
    "witchMagic" => "witch",                      // purple crosses
    "dripWater" => "dripping_water",              // blue circles stuck for a second, then falling
    "dripLava" => "dripping_lava",                // dripping_water but with orange circles
    "angryVillager" => "angry_villager",          // shown when you attack a villager
    "happyVillager" => "happy_villager",          // shown when you trade with a villager
    "townaura" => "mycelium",                     // appears to be the same as mycelium
    "note" => "note",                             // a musical note
    "portal" => "poral",                          // purple particles falling slowly
    "enchantmenttable" => "enchant",              // weird letters floating
    "flame" => "flame",                           // small fire icons
    "lava" => "lava",                             // orange embers with smoke flying out
    "footstep" => return None,                    // doens't exist on new versions
    "cloud" => "cloud",                           // similar to poof
    "reddust" => "dust",                          // redstone dust
    "snowballpoof" => "item_snowball",            // snowball collision particle
    "snowshovel" => "snowflake",                  // small snow particles falling
    "slime" => "item_slime",                      // same as item_snowball, but for slime
    "heart" => "heart",                           // red hearts, used after animals breed
    "barrier" => return None,                     // replaced with block_marker
    "iconcrack_" => return None,                  // doesn't render
    "blockcrack_" => "block",                     // block break particles
    "blockdust_" => "block",                      // particles which appear underfoot
    "droplet" => "rain",                          // rain splash on ground
    "take" => return None,                        // doens't render
    "mobappearance" => "elder_guardian",          // elder guardian appearing onscreen
    _ => return None,
  }
  .map(Into::into)
}
