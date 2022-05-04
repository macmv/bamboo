use crate::{
  item,
  item::{Inventory, Stack},
};
use serde::Deserialize;
use std::{
  collections::HashMap,
  fs, io, mem,
  ops::{Index, IndexMut},
  path::Path,
  str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Recipe {
  items: Grid<Stack>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Grid<T> {
  width:  usize,
  height: usize,
  items:  Vec<T>,
}

pub struct CraftingData {
  recipes: HashMap<Recipe, Stack>,
}

impl Recipe {
  pub fn new() -> Self { Recipe { items: Grid::new(2, 3) } }
}

impl<T: Default + Clone> Grid<T> {
  pub fn new(width: usize, height: usize) -> Self {
    Grid { width, height, items: vec![T::default(); width * height] }
  }
}

impl<T> Grid<T> {
  #[track_caller]
  pub fn set(&mut self, x: usize, y: usize, val: T) { self.items[y * self.width + x] = val; }
}

impl<T> Index<(usize, usize)> for Grid<T> {
  type Output = T;

  fn index(&self, pos: (usize, usize)) -> &T {
    if pos.0 >= self.width || pos.1 >= self.height {
      panic!("index out of grid size {} by {}: {} {}", self.width, self.height, pos.0, pos.1);
    }
    &self.items[pos.1 * self.width + pos.0]
  }
}

impl<T> IndexMut<(usize, usize)> for Grid<T> {
  fn index_mut(&mut self, pos: (usize, usize)) -> &mut T {
    if pos.0 >= self.width || pos.1 >= self.height {
      panic!("index out of grid size {} by {}: {} {}", self.width, self.height, pos.0, pos.1);
    }
    &mut self.items[pos.1 * self.width + pos.0]
  }
}

/// Shaped:
/// ```json
/// {
///   "type": "minecraft:crafting_shaped",
///   "pattern": [
///     "III",
///     " i ",
///     "iii"
///   ],
///   "key": {
///     "I": {
///       "item": "minecraft:iron_block"
///     },
///     "i": {
///       "item": "minecraft:iron_ingot"
///     }
///   },
///   "result": {
///     "item": "minecraft:anvil"
///   }
/// }
/// ```
/// Shapeless:
/// ```json
///  {
///   "type": "minecraft:crafting_shapeless",
///   "ingredients": [
///     {
///       "item": "minecraft:gunpowder"
///     },
///     {
///       "item": "minecraft:paper"
///     }
///   ],
///   "result": {
///     "item": "minecraft:firework_rocket",
///     "count": 3
///   }
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum JsonRecipe {
  #[serde(rename = "minecraft:crafting_shaped")]
  CraftingShaped { pattern: Vec<String>, key: HashMap<char, CraftingKey>, result: JsonItem },
  #[serde(rename = "minecraft:crafting_shapeless")]
  CraftingShapeless { ingredients: Vec<CraftingKey>, result: JsonItem },
  #[serde(rename = "minecraft:smelting")]
  Smelting {},
  #[serde(rename = "minecraft:smoking")]
  Smoking {},
  #[serde(rename = "minecraft:blasting")]
  Blasting {},
  #[serde(rename = "minecraft:stonecutting")]
  Stonecutting {},
  #[serde(rename = "minecraft:smithing")]
  Smithing {},
  #[serde(rename = "minecraft:campfire_cooking")]
  CampfireCooking {},

  /// Special types:
  /// ```json
  /// "minecraft:crafting_special_armordye"
  /// "minecraft:crafting_special_bannerduplicate"
  /// "minecraft:crafting_special_bookcloning"
  /// "minecraft:crafting_special_firework_rocket"
  /// "minecraft:crafting_special_firework_star"
  /// "minecraft:crafting_special_firework_star_fade"
  /// "minecraft:crafting_special_mapcloning"
  /// "minecraft:crafting_special_mapextending"
  /// "minecraft:crafting_special_repairitem"
  /// "minecraft:crafting_special_shielddecoration"
  /// "minecraft:crafting_special_shulkerboxcoloring"
  /// "minecraft:crafting_special_suspiciousstew"
  /// "minecraft:crafting_special_tippedarrow"
  /// ```
  #[serde(other)]
  Special,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum CraftingKey {
  Any(Vec<JsonItem>),
  Single(JsonItem),
}

#[derive(Debug, Clone, Deserialize)]
struct JsonItem {
  item:  Option<String>,
  tag:   Option<String>,
  count: Option<u8>,
}

impl CraftingData {
  pub fn load(path: &Path) -> Self {
    let mut data = CraftingData { recipes: HashMap::new() };
    match data.read(path) {
      Ok(_) => {}
      Err(e) => {
        error!("error loading data: {e}");
      }
    }
    data
  }

  fn read(&mut self, path: &Path) -> io::Result<()> {
    for ent in fs::read_dir(path)? {
      let entry = ent?;
      let source = fs::read_to_string(&entry.path())?;
      let recipe: JsonRecipe = serde_json::from_str(&source).unwrap();
      if let Some((recipe, output)) = recipe.into_recipe() {
        self.recipes.insert(recipe, output);
      }
    }
    Ok(())
  }
  pub fn recipe(&self, recipe: &Recipe) -> Option<&Stack> { self.recipes.get(recipe) }

  pub fn craft(&self, input: &Inventory<9>) -> Option<Stack> {
    info!("crafting {:#?}", input);
    let mut width = 3;
    let mut height = 3;
    let mut grid = Grid::new(3, 3);
    for (i, it) in input.items().iter().enumerate() {
      grid.items[i] = it.clone();
    }
    while height > 0 {
      if grid[(0, 0)].is_empty() && grid[(1, 0)].is_empty() && grid[(2, 0)].is_empty() {
        for y in 1..3 {
          for x in 0..3 {
            grid[(x, y - 1)] = mem::replace(&mut grid[(x, y)], Stack::empty());
          }
        }
        height -= 1;
      } else if grid[(0, height - 1)].is_empty()
        && grid[(1, height - 1)].is_empty()
        && grid[(2, height - 1)].is_empty()
      {
        height -= 1;
      } else {
        break;
      }
    }
    while width > 0 {
      if grid[(0, 0)].is_empty() && grid[(0, 1)].is_empty() && grid[(0, 2)].is_empty() {
        for x in 1..3 {
          for y in 0..3 {
            grid[(x - 1, y)] = mem::replace(&mut grid[(x, y)], Stack::empty());
          }
        }
        width -= 1;
      } else if grid[(width - 1, 0)].is_empty()
        && grid[(width - 1, 1)].is_empty()
        && grid[(width - 1, 2)].is_empty()
      {
        width -= 1;
      } else {
        break;
      }
    }
    let mut subsection = Grid::new(width, height);
    for x in 0..width {
      for y in 0..height {
        subsection.set(x, y, grid[(x, y)].clone());
      }
    }
    info!("got subsection {subsection:#?}");
    let recipe = Recipe { items: subsection };
    self.recipes.get(&recipe).cloned()
  }
}

impl JsonRecipe {
  pub fn into_recipe(self) -> Option<(Recipe, Stack)> {
    Some(match self {
      JsonRecipe::CraftingShaped { pattern, key, result } => {
        (Recipe::parse_shaped(&pattern, key)?, result.into_stack()?)
      }
      _ => return None,
    })
  }
}

impl JsonItem {
  pub fn into_stack(self) -> Option<Stack> {
    let ty = item::Type::from_str(self.item?.strip_prefix("minecraft:")?).ok()?;
    Some(Stack::new(ty).with_amount(self.count.unwrap_or(1)))
  }
}
impl Recipe {
  fn parse_shaped(pattern: &[String], key: HashMap<char, CraftingKey>) -> Option<Self> {
    let mut grid = Grid::new(pattern[0].len(), pattern.len());

    for (i, row) in pattern.iter().enumerate() {
      for (j, c) in row.chars().enumerate() {
        if c != ' ' {
          grid.set(j, i, key[&c].to_stack()?);
        }
      }
    }

    Some(Recipe { items: grid })
  }
}

impl CraftingKey {
  pub fn to_stack(&self) -> Option<Stack> {
    match self {
      Self::Single(item) => item.clone().into_stack(),
      _ => None,
    }
  }
}
