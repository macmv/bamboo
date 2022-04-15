use crate::world::gen::PdBiomeGen;
use panda::define_ty;
use std::{fmt, sync::Arc};

#[derive(Clone)]
#[allow(unused)]
pub struct PdBiome {
  name:             String,
  pub(super) inner: Arc<PdBiomeGen>,
}

impl fmt::Debug for PdBiome {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("PdBiome").field("name", &self.name).finish()
  }
}

/// A biome. This is how you can modify terrain generation.
///
/// # Example
///
/// ```
/// fn main(bb) {
///   biome = Biome::new("desert")
///   // First, we make the desert have a sand ground.
///
///   // This is the ground of our biome. The parameter passed here is the
///   // bottom-most layer of the ground, which we usually want to be stone.
///   layers = Layers::new("stone")
///   // This adds a new layer ontop of the stone, which will be 5 blocks
///   // of sandstone.
///   layers.add("sandstone", 5)
///   // This adds our topmost layer, which will be 3 blocks of sand.
///   layers.add("sand", 3)
///   // By default, the biome uses layers of dirt and grass. For something
///   // like a forest, we wouldn't need to change the layers at all.
///   biome.use_layers(layers)
///
///   // Second, we add dead bushes.
///
///   // Every feature needs a structure. Structures can be simple (single
///   // blocks like flowers or grass), or they can be complex (like trees).
///   // Dead bushes are very simple, so we just want a single block structure.
///   dead_bush_struct = Structure::from_block("dead_bush")
///   // Plant features are common. They can be used to generate clumps of
///   // grass, flowers, dead bushes, cactuses, and the like. Because the
///   // shape of the structure can be determined at any time, these can
///   // also even be used to generate trees. Things like villages require
///   // a new feature, as they have a very complex layout.
///   //
///   // The first argument is our Structure. The second argument is the size
///   // of each clump. This is used for flowers and grass, to add some variety
///   // to the terrain. The third argument is the average distance between
///   // each clump. For something like flowers, we might have clumps of 20,
///   // and a distance of 50 blocks or so.
///   dead_bush = PlantFeature::new(dead_bush_struct, 1, 10)
///   // Biomes have no features by default. This will make our biome generate
///   // dead bushes.
///   biome.add_feature(dead_bush)
///
///   // Third, we add cacti.
///
///   // Cactuses are more complex than dead bushes. We want them to vary in
///   // height, but other than that they are just a pillar. This means calling
///   // a function every time we place a cactus is needed.
///   //
///   // The second argument here is the radius outside of the center block that
///   // this structure needs. This is mostly for trees. A radius of zero means
///   // that we have a single column. This also prevents the `place_cactus`
///   // function from being passed coordinates outside the chunk.
///   cacus_struct = Structure::from_func(place_cactus, 0)
///   // We want cactuses to be about as common as dead bushes.
///   dead_bush = PlantFeature::new(cactus_struct, 1, 10)
///   // Add the feature like we did before. This order technically matters, as
///   // all features are processed in order. So if a dead bush generates at the
///   // same block as a cactus, the cactus will override the dead bush.
///   biome.add_feature(dead_bush)
///
///   // Finally, add the biome to the terrain generator.
///   bb.add_biome(biome)
/// }
///
/// // A structure will be passed the terrain generator, the chunk that it needs
/// // to place in, and the coordinate of the structure relative to the chunk. For
/// // trees, the leaves may extend into nearby chunks. So, depending on how you
/// // created the Structure, `pos` may not be within the chunk.
/// //
/// // In our example, `pos` will always be inside the chunk. They will be
/// // relative coordinates in the chunk, so this will not change at all for
/// // different places in the world.
/// fn place_cactus(gen, chunk, pos) {
///   // Get a random number between 3 (inclusive) and 6 (exclusive). Note that
///   // this will never return 6! It will always return 3, 4, or 5.
///   height = gen.rand_int(3, 6);
///
///   // Finally, place our cactus.
///   chunk.fill(pos, pos + Pos::new(0, height, 0), "cactus");
/// }
/// ```
#[define_ty(path = "bamboo::world::gen::Biome")]
impl PdBiome {}
