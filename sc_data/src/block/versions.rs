use super::{Block, BlockVersion, State};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::{collections::HashMap, convert::TryInto};

fn generate_version_lit(to_old: Vec<u32>, ver: &str) -> TokenStream {
  let mut to_new: Vec<u32> = vec![];
  for (new, &old) in to_old.iter().enumerate() {
    let old: usize = old.try_into().unwrap();
    if old >= to_new.len() {
      to_new.resize(old + 1, 0);
    }
    // Sometimes, multiple new blocks map to a single old block. In these
    // situations, we want to just use the first state that was mapped. So we never
    // override anything that has a value != 0.
    if to_new[old] == 0 {
      to_new[old] = new.try_into().unwrap();
    }
  }
  let ver = Ident::new(ver, Span::call_site());
  quote! {
    Version {
      to_old: &[#(#to_old),*],
      to_new: &[#(#to_new),*],
      ver: common::version::BlockVersion::#ver,
    }
  }
}

// Called on 1.13+
pub(super) fn generate(latest: &BlockVersion, old: &BlockVersion) -> TokenStream {
  let mut to_old = vec![];

  let old_blocks: HashMap<String, Block> =
    old.blocks.iter().cloned().map(|b| (b.name().to_string(), b)).collect();

  for b in &latest.blocks {
    let old_block = match old_blocks.get(b.name()) {
      Some(v) => v,
      None => &old.blocks[0], // Use air when we there is a missing block
    };
    if b.states().is_empty() {
      to_old.push(old_block.id());
    } else {
      for s in b.states() {
        let mut old_id = 0;
        for o in old_block.states() {
          if o.props() == s.props() {
            old_id = o.id();
            break;
          }
        }
        to_old.push(old_id);
      }
    }
  }

  generate_version_lit(to_old, &old.enum_name)
}

// Called on 1.8-1.12
pub(super) fn generate_old(latest: &BlockVersion, old: &BlockVersion) -> TokenStream {
  // Map of new block names to old block names and metadata values
  let names: HashMap<String, (&str, u32)> = include_str!("old_names.txt")
    .trim()
    .split('\n')
    .map(|l| {
      if l.is_empty() || l.starts_with('#') {
        return vec![];
      }
      // This is the new block and old block
      let sections: Vec<&str> = l.split(':').map(|s| s.trim()).collect();
      // This is the old block name and optional metadata
      let right_sections: Vec<&str> = sections[1].split(' ').collect();
      let mut values = vec![];
      let old_name = if right_sections.len() == 1 {
        (right_sections[0], 0)
      } else {
        (right_sections[0], right_sections[1].parse().unwrap())
      };
      if sections[0].contains("{color}") {
        values.push((sections[0].replace("{color}", "white"), old_name));
        values.push((sections[0].replace("{color}", "orange"), old_name));
        values.push((sections[0].replace("{color}", "magenta"), old_name));
        values.push((sections[0].replace("{color}", "light_blue"), old_name));
        values.push((sections[0].replace("{color}", "yellow"), old_name));
        values.push((sections[0].replace("{color}", "lime"), old_name));
        values.push((sections[0].replace("{color}", "pink"), old_name));
        values.push((sections[0].replace("{color}", "gray"), old_name));
        values.push((sections[0].replace("{color}", "light_gray"), old_name));
        values.push((sections[0].replace("{color}", "cyan"), old_name));
        values.push((sections[0].replace("{color}", "purple"), old_name));
        values.push((sections[0].replace("{color}", "blue"), old_name));
        values.push((sections[0].replace("{color}", "brown"), old_name));
        values.push((sections[0].replace("{color}", "green"), old_name));
        values.push((sections[0].replace("{color}", "red"), old_name));
        values.push((sections[0].replace("{color}", "black"), old_name));
      } else {
        values.push((sections[0].into(), old_name))
      }
      values
    })
    .flatten()
    .collect();

  // The keys in names are prop strings, so we need to convert them into States
  let mut state_maps: HashMap<String, Vec<(Vec<(String, String)>, String, u32)>> = HashMap::new();
  for (key, (old_name, old_meta)) in names {
    let mut iter = key.split('[');
    let name = iter.next().unwrap().to_string();
    let mut props = HashMap::new();
    if let Some(mut prop_str) = iter.next() {
      prop_str = prop_str.split(']').next().unwrap();
      for pair in prop_str.split(',') {
        let mut iter = pair.split('=');
        let key = iter.next().unwrap().trim();
        let val = iter.next().unwrap().trim();

        props.insert(key.to_string(), val.to_string());
      }
    }
    // Props is an incomplete list of properties for the block. We want to say
    // that all possible blocks with those properties have the given old
    // block name/id let key = State { id: blocks[name], properties };

    if let Some(values) = state_maps.get_mut(&name) {
      values.push((props.into_iter().collect(), old_name.into(), old_meta));
    } else {
      state_maps.insert(name, vec![(props.into_iter().collect(), old_name.into(), old_meta)]);
    }
  }

  let mut to_old = vec![];

  // Need to iterate in order
  for b in &latest.blocks {
    match state_maps.get(b.name()) {
      Some(old_values) => {
        if b.states().is_empty() {
          if let Some((_, old_name, old_meta)) = old_values.first() {
            if old_values.len() != 1 {
              eprintln!("FOUND MULTIPLE OLD ID for block {}", b.name());
              eprintln!("this block only has one state: {}", b.name());
            }
            to_old.push(old.get(old_name).unwrap().id() | old_meta);
          } else {
            to_old.push(0);
          }
        } else {
          for state in b.states() {
            let mut old_id = 0;
            for (props, old_name, old_meta) in old_values {
              if state.matches(&props) {
                if old_id != 0 {
                  eprintln!(
                    "FOUND MULTIPLE OLD ID {} for state {}",
                    old_id,
                    state.prop_str(b.name())
                  );
                  panic!();
                }
                old_id = old.get(old_name).unwrap().id() | old_meta;
              }
            }
            if old_id == 0 {
              eprintln!("DID NOT FIND old id for block {}", state.prop_str(b.name()));
              eprintln!("given states:");
              for (props, old_name, old_meta) in old_values {
                let s = State::new(0, props.clone());
                eprintln!("{} -> {} {}", s.prop_str(b.name()), old_name, old_meta);
              }
              eprintln!("possible states:");
              for s in b.states() {
                eprintln!("{}", s.prop_str(b.name()));
              }
              panic!();
            }
            to_old.push(old_id);
          }
        }
      }
      None => {
        let mut old_id = 0;
        if let Some(old) = old.get(b.name()) {
          old_id = old.id();
        } else {
          // eprintln!("missing old id for {} ({})", b.name(), b.id());
        }
        if b.states().is_empty() {
          to_old.push(old_id);
        } else {
          for _ in b.states() {
            to_old.push(old_id);
          }
        }
      }
    }
  }

  generate_version_lit(to_old, &old.enum_name)
}
