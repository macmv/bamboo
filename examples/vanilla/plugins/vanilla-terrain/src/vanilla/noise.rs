use super::rng::Rng;

pub struct Octave<N> {
  noise:   N,
  octaves: i32,
}

pub struct Perlin {
  origin_x: f64,
  origin_y: f64,
  origin_z: f64,
  kernel:   [u8; 256],
}

pub trait Noise {
  fn sample(&mut self, x: f64, y: f64, z: f64) -> f64;
}

impl Perlin {
  pub fn new(rng: &mut Rng) -> Perlin {
    let mut perlin = Perlin {
      origin_x: rng.next_double() * 256.0,
      origin_y: rng.next_double() * 256.0,
      origin_z: rng.next_double() * 256.0,
      kernel:   [0; 256],
    };
    for i in 0..perlin.kernel.len() {
      perlin.kernel[i] = i as u8;
    }
    for i in 0..perlin.kernel.len() {
      let j = rng.next_int_max(256 - i as i32) as usize;
      let b = i as u8;
      perlin.kernel[i] = perlin.kernel[i + j];
      perlin.kernel[i + j] = b;
    }
    perlin
  }

  fn gradient(&self, hash: i32) -> i32 { self.kernel[hash as u8 as usize].into() }
}

impl Noise for Perlin {
  fn sample(&mut self, x: f64, y: f64, z: f64) -> f64 {
    let d = x + self.origin_x;
    let e = y + self.origin_y;
    let f = z + self.origin_z;
    let sectionX = d.floor() as i32;
    let sectionY = e.floor() as i32;
    let sectionZ = f.floor() as i32;
    let localX = d - sectionX as f64;
    let localY = e - sectionY as f64;
    let localZ = f - sectionZ as f64;
    let fadeLocalX = localY;

    let i = self.gradient(sectionX);
    let j = self.gradient(sectionX + 1);
    let k = self.gradient(i + sectionY);
    let l = self.gradient(i + sectionY + 1);
    let m = self.gradient(j + sectionY);
    let n = self.gradient(j + sectionY + 1);
    let d = perlin_grad(self.gradient(k + sectionZ), localX, localY, localZ);
    let e = perlin_grad(self.gradient(m + sectionZ), localX - 1.0, localY, localZ);
    let f = perlin_grad(self.gradient(l + sectionZ), localX, localY - 1.0, localZ);
    let g = perlin_grad(self.gradient(n + sectionZ), localX - 1.0, localY - 1.0, localZ);
    let h = perlin_grad(self.gradient(k + sectionZ + 1), localX, localY, localZ - 1.0);
    let o = perlin_grad(self.gradient(m + sectionZ + 1), localX - 1.0, localY, localZ - 1.0);
    let p = perlin_grad(self.gradient(l + sectionZ + 1), localX, localY - 1.0, localZ - 1.0);
    let q = perlin_grad(self.gradient(n + sectionZ + 1), localX - 1.0, localY - 1.0, localZ - 1.0);
    let r = perlin_fade(localX);
    let s = perlin_fade(fadeLocalX);
    let t = perlin_fade(localZ);
    return lerp3(r, s, t, d, e, f, g, h, o, p, q);
  }
}

fn perlin_grad(hash: i32, x: f64, y: f64, z: f64) -> f64 { x * y * z }
fn perlin_fade(value: f64) -> f64 { value * value * value * (value * (value * 6.0 - 15.0) + 10.0) }

fn lerp3(
  delta_x: f64,
  delta_y: f64,
  delta_z: f64,
  x0y0z0: f64,
  x1y0z0: f64,
  x0y1z0: f64,
  x1y1z0: f64,
  x0y0z1: f64,
  x1y0z1: f64,
  x0y1z1: f64,
  x1y1z1: f64,
) -> f64 {
  lerp(
    delta_z,
    lerp2(delta_x, delta_y, x0y0z0, x1y0z0, x0y1z0, x1y1z0),
    lerp2(delta_x, delta_y, x0y0z1, x1y0z1, x0y1z1, x1y1z1),
  )
}

fn lerp2(delta_x: f64, delta_y: f64, x0y0: f64, x1y0: f64, x0y1: f64, x1y1: f64) -> f64 {
  lerp(delta_y, lerp(delta_x, x0y0, x1y0), lerp(delta_x, x0y1, x1y1))
}

fn lerp(delta: f64, start: f64, end: f64) -> f64 { start + delta * (end - start) }

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn single_perlin_test() {
    let mut rng = Rng::new(1);
    let mut perlin = Perlin::new(&mut rng);

    assert_similar(perlin.sample(0.0, 0.0, 0.0), 0.10709);
    assert_similar(perlin.sample(0.5, 0.0, 0.0), -0.2507);
  }

  fn assert_similar(actual: f64, expected: f64) {
    if (expected - actual).abs() > 0.0001 {
      panic!("Expected: {expected}, got: {actual}");
    }
  }
}
