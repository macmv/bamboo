use bb_plugin::{
  chunk::{paletted, Chunk, Section},
  math::{ChunkPos, Pos, SectionRelPos},
};
use std::sync::Arc;

pub mod density;
pub mod noise;
pub mod noise_params;
pub mod rng;

use noise::Noise;
use rng::SimpleRng;

use density::{Density, DensityFunc, NoisePos, World};

pub fn generate_chunk(chunk: &mut Chunk<paletted::Section>, pos: ChunkPos) {
  let mut rng = SimpleRng::new(SEED);
  let gen = NoiseGenerator {
    vertical_size:   1,
    horizontal_size: 2,
    noise:           World::new(&mut rng),
  };
  let chunk = gen.populate_noise(chunk, pos);
}

const SEED: i64 = -2238292588208479879;

struct NoiseGenerator {
  noise:               World,
  pub vertical_size:   i32,
  pub horizontal_size: i32,
}
struct ChunkNoiseSampler<'a> {
  gen:           &'a NoiseGenerator,
  block_sampler: BlockStateSampler,

  chunk_x:  i32,
  chunk_z:  i32,
  offset_x: i32,
  offset_y: i32,
  offset_z: i32,
  x:        i32,
  y:        i32,
  z:        i32,
}
struct BlockStateSampler {
  density: Arc<DensityFunc>,
}

impl BlockStateSampler {
  // Returns a block state, for the given position.
  fn sample(&self, pos: NoisePos) -> u32 {
    if self.density.sample(pos) > 0.0 {
      0
    } else {
      1
    }
  }
}

impl<'a> ChunkNoiseSampler<'a> {
  pub fn new(chunk_x: i32, chunk_z: i32, gen: &'a NoiseGenerator) -> Self {
    ChunkNoiseSampler {
      gen,
      chunk_x,
      chunk_z,
      block_sampler: BlockStateSampler { density: gen.noise.density_funcs.final_density.clone() },
      offset_x: 0,
      offset_y: 0,
      offset_z: 0,
      x: 0,
      y: 0,
      z: 0,
    }
  }
  fn sample_end_noise(&mut self, x: i32) {
    self.offset_x = (self.chunk_x + x) * self.gen.horizontal_size;
  }
  fn sample_noise_corners(&mut self, y: i32, z: i32) {
    const MIN_Y: i32 = 0;

    self.offset_y = (y + MIN_Y) * self.gen.vertical_size;
    self.offset_z = (self.chunk_z + z) * self.gen.horizontal_size;
    /*
    this.field_36593 = true;
    ++this.field_36578;
    for (class_6949 lv : this.field_36581) {
      lv.field_36603.method_40470(lv.field_36604, this);
    }
    ++this.field_36578;
    this.field_36593 = false;
    */
  }
  fn sample_noise_x(&mut self, block_x: i32, x: f64) { self.x = block_x - self.offset_x; }
  fn sample_noise_y(&mut self, block_y: i32, y: f64) { self.y = block_y - self.offset_y; }
  // ++this.field_36577;
  fn sample_noise_z(&mut self, block_z: i32, z: f64) { self.z = block_z - self.offset_z; }

  fn as_noise_pos(&self) -> NoisePos {
    NoisePos { x: self.offset_x + self.x, y: self.offset_y + self.y, z: self.offset_z + self.z }
  }

  fn get_block_inner(&mut self, x: i32, y: i32, z: i32, noise: f64) -> f64 {
    let e = 0.0;
    let mut f = 0.0;
    let mut g = 0.0;
    /*
    f = if bl2 { this.method_38409(point.peaks(), x as f64, z as f64) } else { 0.0 };
    g = (this.method_39331(y, point) + f) * point.factor();
    e = g * if g > 0.0 { 4.0 } else { 1.0 };
    */

    f = e + noise;
    g = 1.5625;
    let h;
    let l;
    let m;
    let mut n;
    let has_no_noise_caves = true;
    if !has_no_noise_caves && !(f < -64.0) {
      m = 0.0;
      h = 0.0;
      l = 0.0;
      /*
      n = f - 1.5625;
      let bl3 = n < 0.0;
      let o = this.sampleCaveEntranceNoise(x, y, z);
      let p = this.sampleSpaghettiRoughnessNoise(x, y, z);
      let q = this.sampleSpaghetti3dNoise(x, y, z);
      let r = Math.min(o, q + p);
      if bl3 {
        h = f;
        l = r * 5.0;
        m = -64.0;
      } else {
        let s = this.sampleCaveLayerNoise(x, y, z);
        let t;
        if (s > 64.0) { h = 64.0;
        } else {
          t = this.caveCheeseNoise.sample(x as f64, y as f64 / 1.5, z as f64);
          let u = MathHelper.clamp(t + 0.27, -1.0, 1.0);
          let v = n * 1.28;
          let w = u + MathHelper.clampedLerp(0.5, 0.0, v);
          h = w + s;
        }

        t = this.sampleSpaghetti2dNoise(x, y, z);
        l = Math.min(r, t + p);
        m = this.samplePillarNoise(x, y, z);
      }
      */
    } else {
      h = f;
      l = 64.0;
      m = -64.0;
    }

    n = if h < l { h } else { l };
    n = if n > m { n } else { m };
    // n = self.apply_slides(n, y / self.vertical_size);
    // n = blender.method_39338(x, y, z, n);
    if n < -64.0 {
      -64.0
    } else if n > 64.0 {
      64.0
    } else {
      n
    }
  }

  fn sample_block(&mut self) -> u32 {
    self.block_sampler.sample(self.as_noise_pos())
    /*
    // let d = self.get_block_inner(x, y, z, x as f64 / 100.0) / 500.0;
    let mut e = d * 0.64;
    if e < -1.0 {
      e = -1.0;
    }
    if e > 1.0 {
      e = 1.0;
    }
    e = e / 2.0 - e * e * e / 24.0;
    /*
    // Caves
    if (lv2.sample() >= 0.0) {
       double f = 0.05;
       double g = 0.1;
       double h = MathHelper.clampedLerpFromProgress(lv3.sample(), -1.0, 1.0, 0.05, 0.1);
       double l = Math.abs(1.5 * lv4.sample()) - h;
       double m = Math.abs(1.5 * lv5.sample()) - h;
       e = Math.min(e, Math.max(l, m));
    }
    */

    /*
    e += columnSampler.calculateNoise(x, y, z);
    return chunkNoiseSampler.getAquiferSampler().apply(x, y, z, d, e);
    */
    if e > y as f64 / 32.0 {
      1
    } else {
      0
    }
    */
  }
}

impl NoiseGenerator {
  fn populate_noise(&self, chunk: &mut Chunk<paletted::Section>, pos: ChunkPos) {
    let mut sampler = ChunkNoiseSampler::new(pos.x(), pos.z(), self);

    const MIN_Y: i32 = 0;
    const MAX_Y: i32 = 256;

    let mut total_sample = 0;
    let mut total_setblock = 0;
    let now = bb_plugin::time::Instant::now();

    for section_x in 0..(16 / self.horizontal_size) {
      sampler.sample_end_noise(section_x);
      for section_z in 0..(16 / self.horizontal_size) {
        let mut section = chunk.section_mut(15);
        let mut chunk_section_y = 15;
        for section_y in (0..MAX_Y / self.vertical_size).rev() {
          sampler.sample_noise_corners(section_y, section_z);

          for loop_y in (0..self.vertical_size).rev() {
            let y = (MIN_Y / self.vertical_size + section_y) * self.vertical_size + loop_y;
            let rel_y = Pos::new(0, y, 0).chunk_rel_y();
            let inner_chunk_y = Pos::new(0, y, 0).chunk_y();
            if chunk_section_y != inner_chunk_y {
              chunk_section_y = inner_chunk_y;
              section = chunk.section_mut(chunk_section_y as u32);
            }
            sampler.sample_noise_y(y, loop_y as f64 / self.vertical_size as f64);

            for loop_x in 0..self.horizontal_size {
              let x = pos.block_x() + section_x * self.horizontal_size + loop_x;
              let rel_x = Pos::new(x, 0, 0).chunk_rel_x();
              sampler.sample_noise_x(x, loop_x as f64 / self.horizontal_size as f64);

              for loop_z in 0..self.horizontal_size {
                let z = pos.block_z() + section_z * self.horizontal_size + loop_z;
                let rel_z = Pos::new(0, 0, z).chunk_rel_z();
                sampler.sample_noise_z(z, loop_z as f64 / self.horizontal_size as f64);

                let now = bb_plugin::time::Instant::now();
                let block = sampler.sample_block();
                total_sample += now.elapsed().as_nanos();

                let now = bb_plugin::time::Instant::now();
                if block != 0 {
                  /*
                   if (lv10.getLuminance() != 0 && chunk instanceof ProtoChunk) {
                      lv7.set(x, y, ab);
                      ((ProtoChunk)chunk).addLightSource(lv7);
                   }
                  */

                  section.set_block(
                    SectionRelPos::new(
                      rel_x.try_into().unwrap(),
                      rel_y.try_into().unwrap(),
                      rel_z.try_into().unwrap(),
                    ),
                    block,
                  );
                  /*
                  lv3.trackUpdate(z, u, ac, lv10);
                  lv4.trackUpdate(z, u, ac, lv10);
                  if (lv6.needsFluidTick() && !lv10.getFluidState().isEmpty()) {
                    lv7.set(y, u, ab);
                    chunk.markBlockForPostProcessing(lv7);
                  }
                  */
                }
                total_setblock += now.elapsed().as_nanos();
              }
            }
          }
        }
      }
    }
    let total = now.elapsed().as_nanos() as f64 / 1_000_000.0;
    let total_sample = total_sample as f64 / 1_000_000.0;
    let total_setblock = total_setblock as f64 / 1_000_000.0;
    // info!("total: {total:.4}ms");
    // info!("sample: {total_sample:.4}ms ({:.2}%)", total_sample / total *
    // 100.0); info!("setblock: {total_setblock:.4}ms ({:.2}%)",
    // total_setblock / total * 100.0);
    /*

    for(int q = 0; q < o; ++q) {
       lv2.sampleEndNoise(q);

       for(int r = 0; r < p; ++r) {
          ChunkSection lv9 = chunk.getSection(chunk.countVerticalSections() - 1);

          for(int s = j - 1; s >= 0; --s) {
             lv2.sampleNoiseCorners(s, r);

             for(int t = n - 1; t >= 0; --t) {
                int u = (i + s) * n + t;
                int v = u & 15;
                int w = chunk.getSectionIndex(u);
                if (chunk.getSectionIndex(lv9.getYOffset()) != w) {
                   lv9 = chunk.getSection(w);
                }

                double d = (double)t / (double)n;
                lv2.sampleNoiseY(d);

                for(int x = 0; x < m; ++x) {
                   int y = k + q * m + x;
                   int z = y & 15;
                   double e = (double)x / (double)m;
                   lv2.sampleNoiseX(e);

                   for(int aa = 0; aa < m; ++aa) {
                      int ab = l + r * m + aa;
                      int ac = ab & 15;
                      double f = (double)aa / (double)m;
                      lv2.sampleNoise(f);
                      BlockState lv10 = this.blockStateSampler.apply(lv2, y, u, ab);
                      if (lv10 == null) {
                         lv10 = this.defaultBlock;
                      }

                      lv10 = this.getBlockState(lv2, y, u, ab, lv10);
                      if (lv10 != AIR && !SharedConstants.method_37896(chunk.getPos())) {
                         if (lv10.getLuminance() != 0 && chunk instanceof ProtoChunk) {
                            lv7.set(y, u, ab);
                            ((ProtoChunk)chunk).addLightSource(lv7);
                         }

                         lv9.setBlockState(z, v, ac, lv10, false);
                         lv3.trackUpdate(z, u, ac, lv10);
                         lv4.trackUpdate(z, u, ac, lv10);
                         if (lv6.needsFluidTick() && !lv10.getFluidState().isEmpty()) {
                            lv7.set(y, u, ab);
                            chunk.markBlockForPostProcessing(lv7);
                         }
                      }
                   }
                }
             }
          }
       }

       lv2.swapBuffers();
    }

    return chunk;

    ChunkGeneratorSettings lv = (ChunkGeneratorSettings)this.settings.get();
    ChunkNoiseSampler lv2 = chunk.getOrCreateChunkNoiseSampler(this.noiseColumnSampler, () -> {
       return new StructureWeightSampler(structureAccessor, chunk);
    }, lv, this.fluidLevelSampler, blender);
    Heightmap lv3 = chunk.getHeightmap(Heightmap.Type.OCEAN_FLOOR_WG);
    Heightmap lv4 = chunk.getHeightmap(Heightmap.Type.WORLD_SURFACE_WG);
    ChunkPos lv5 = chunk.getPos();
    int k = lv5.getStartX();
    int l = lv5.getStartZ();
    AquiferSampler lv6 = lv2.getAquiferSampler();
    lv2.sampleStartNoise();
    BlockPos.Mutable lv7 = new BlockPos.Mutable();
    GenerationShapeConfig lv8 = lv.getGenerationShapeConfig();
    int m = lv8.horizontalBlockSize();
    int n = lv8.verticalBlockSize();
    int o = 16 / m;
    int p = 16 / m;

    */
  }
}
