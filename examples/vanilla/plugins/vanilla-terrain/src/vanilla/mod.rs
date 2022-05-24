use bb_plugin::{
  chunk::{paletted, Chunk, Section},
  math::{ChunkPos, Pos, SectionRelPos},
};

pub mod density_funcs;
pub mod noise;
pub mod noise_params;
pub mod rng;

use noise::Noise;
use rng::Rng;

pub fn generate_chunk(chunk: &mut Chunk<paletted::Section>, pos: ChunkPos) {
  let mut rng = Rng::new(SEED);
  let gen = NoiseGenerator {
    vertical_size:   1,
    horizontal_size: 2,
    noise:           density_funcs::World::new(&mut rng),
  };
  gen.populate_noise(chunk, pos);
}

const SEED: i64 = -2238292588208479879;

struct NoiseGenerator {
  noise:               density_funcs::World,
  pub vertical_size:   i32,
  pub horizontal_size: i32,
}
struct NoiseSampler<'a> {
  gen: &'a NoiseGenerator,
}

impl NoiseSampler<'_> {
  fn sample_end_noise(&mut self, v: i32) {}
  fn sample_noise_corners(&mut self, x: i32, z: i32) {}
  fn sample_noise_x(&mut self, v: f64) {}
  fn sample_noise_y(&mut self, v: f64) {}
  fn sample_noise(&mut self, v: f64) {}

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
        if (s > 64.0) {
          h = 64.0;
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
    let _ = if n < -64.0 {
      -64.0
    } else if n > 64.0 {
      64.0
    } else {
      n
    };
    self.gen.noise.sample(x as f64, y as f64, z as f64)
  }

  fn get_block(&mut self, x: i32, y: i32, z: i32) -> u32 {
    let d = self.get_block_inner(x, y, z, x as f64 / 100.0) / 100.0;
    if d + 64.0 > y as f64 {
      1
    } else {
      0
    }
    /*
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
    let m = self.horizontal_size;
    let n = self.vertical_size;
    let o = 16 / m;
    let p = 16 / m;
    let mut sampler = NoiseSampler { gen: self };
    let i = 0 / self.vertical_size;
    let j = 256 / self.vertical_size;

    for q in 0..o {
      sampler.sample_end_noise(q);
      for r in 0..p {
        let mut section = chunk.section_mut(15);
        let mut section_y = 15;
        for s in (0..=j - 1).rev() {
          sampler.sample_noise_corners(s, r);

          for t in (0..=n - 1).rev() {
            let u = (i + s) * n + t;
            let v = u & 15;
            let w = Pos::new(0, u, 0).chunk_y();
            if section_y != w {
              section_y = w;
              section = chunk.section_mut(section_y as u32);
            }
            let d = t as f64 / n as f64;
            sampler.sample_noise_y(d);

            for x in 0..m {
              let y = pos.block_x() + q * m + x;
              let z = y & 15;
              let e = x as f64 / m as f64;
              sampler.sample_noise_x(e);

              for aa in 0..m {
                let ab = pos.block_z() + r * m + aa;
                let ac = ab & 15;
                let f = aa as f64 / m as f64;
                sampler.sample_noise(f);
                let block = sampler.get_block(y, u, ab);

                if block != 0 {
                  /*
                   if (lv10.getLuminance() != 0 && chunk instanceof ProtoChunk) {
                      lv7.set(y, u, ab);
                      ((ProtoChunk)chunk).addLightSource(lv7);
                   }
                  */

                  section.set_block(
                    SectionRelPos::new(
                      z.try_into().unwrap(),
                      v.try_into().unwrap(),
                      ac.try_into().unwrap(),
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
              }
            }
          }
        }
      }
    }
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
