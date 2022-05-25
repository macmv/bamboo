use super::{
  super::density_funcs::{Density, NoisePos},
  Noise, NoiseConfig, OctavePerlin,
};

pub struct Interpolated {
  lower:         OctavePerlin,
  upper:         OctavePerlin,
  interp:        OctavePerlin,
  xz_scale:      f64,
  y_scale:       f64,
  xz_main_scale: f64,
  y_main_scale:  f64,
  cell_width:    i32,
  cell_height:   i32,
}

impl Interpolated {
  pub fn new(
    lower: OctavePerlin,
    upper: OctavePerlin,
    interp: OctavePerlin,
    cell_width: i32,
    cell_height: i32,
    config: &NoiseConfig,
  ) -> Self {
    let xz_scale = 684.412 * config.xz_scale;
    let y_scale = 684.412 * config.y_scale;
    Interpolated {
      lower,
      upper,
      interp,
      xz_scale,
      y_scale,
      xz_main_scale: xz_scale * config.xz_factor,
      y_main_scale: y_scale * config.y_factor,
      cell_width,
      cell_height,
    }
  }
}

fn floor_div(x: i32, y: i32) -> i32 {
  let r = x / y;
  // if the signs are different and modulo not zero, round down
  if (x ^ y) < 0 && (r * y != x) {
    r - 1
  } else {
    r
  }
}

impl Density for Interpolated {
  fn sample(&self, pos: NoisePos) -> f64 {
    use super::octave::maintain_precision;
    let i = floor_div(pos.x, self.cell_width);
    let j = floor_div(pos.y, self.cell_height);
    let k = floor_div(pos.z, self.cell_width);
    let mut total = 0.0;
    let mut persistence = 1.0;
    for octave in 0..8 {
      let perlin = self.interp.get_octave(octave);
      total += perlin.sample_scale(
        maintain_precision(i as f64 * self.xz_main_scale * persistence),
        maintain_precision(j as f64 * self.y_main_scale * persistence),
        maintain_precision(k as f64 * self.xz_main_scale * persistence),
        self.y_main_scale * persistence,
        j as f64 * self.y_main_scale * persistence,
      ) / persistence;
      persistence /= 2.0;
    }
    let mut mapped = (total / 10.0 + 1.0) / 2.0;
    let bl2 = mapped >= 1.0;
    let bl3 = mapped <= 0.0;
    let mut persistence = 1.0;
    let mut lower = 0.0;
    let mut upper = 0.0;
    for octave in 0..16 {
      let n = maintain_precision(i as f64 * self.xz_scale * persistence);
      let o = maintain_precision(j as f64 * self.y_scale * persistence);
      let p = maintain_precision(k as f64 * self.xz_scale * persistence);
      let scale_y = self.y_scale * persistence;
      if !bl2 {
        let perlin = self.lower.get_octave(octave);
        lower += perlin.sample_scale(n, o, p, scale_y, j as f64 * scale_y) / persistence;
      }
      if !bl3 {
        let perlin = self.upper.get_octave(octave);
        upper += perlin.sample_scale(n, o, p, scale_y, j as f64 * scale_y) / persistence;
      }
      persistence /= 2.0;
    }
    let start = lower / 512.0;
    let end = upper / 512.0;
    if mapped < 0.0 {
      mapped = start;
    }
    if mapped > 1.0 {
      mapped = end
    }
    super::perlin::lerp(mapped, start, end) / 128.0
  }
}
