use super::{super::rng::Rng, Noise};

pub struct Perlin {
  origin_x: f64,
  origin_y: f64,
  origin_z: f64,
  kernel:   [i8; 256],
}

impl Perlin {
  pub fn new<R: Rng>(rng: &mut R) -> Perlin {
    let mut perlin = Perlin {
      origin_x: rng.next_double() * 256.0,
      origin_y: rng.next_double() * 256.0,
      origin_z: rng.next_double() * 256.0,
      kernel:   [0; 256],
    };
    for i in 0..perlin.kernel.len() {
      perlin.kernel[i] = i as i8;
    }
    for i in 0..perlin.kernel.len() {
      let j = rng.next_int_max(256 - i as i32) as usize;
      perlin.kernel.swap(i, i + j);
    }
    perlin
  }

  fn gradient(&self, hash: i32) -> i32 { self.kernel[(hash & 0xff) as usize].into() }
}

impl Noise for Perlin {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
    let d = x + self.origin_x;
    let e = y + self.origin_y;
    let f = z + self.origin_z;
    let section_x = d.floor() as i32;
    let section_y = e.floor() as i32;
    let section_z = f.floor() as i32;
    let local_x = d - section_x as f64;
    let local_y = e - section_y as f64;
    let local_z = f - section_z as f64;
    let fade_local_x = local_y;

    let i = self.gradient(section_x);
    let j = self.gradient(section_x + 1);
    let k = self.gradient(i + section_y);
    let l = self.gradient(i + section_y + 1);
    let m = self.gradient(j + section_y);
    let n = self.gradient(j + section_y + 1);
    let d = perlin_grad(self.gradient(k + section_z), local_x, local_y, local_z);
    let e = perlin_grad(self.gradient(m + section_z), local_x - 1.0, local_y, local_z);
    let f = perlin_grad(self.gradient(l + section_z), local_x, local_y - 1.0, local_z);
    let g = perlin_grad(self.gradient(n + section_z), local_x - 1.0, local_y - 1.0, local_z);
    let h = perlin_grad(self.gradient(k + section_z + 1), local_x, local_y, local_z - 1.0);
    let o = perlin_grad(self.gradient(m + section_z + 1), local_x - 1.0, local_y, local_z - 1.0);
    let p = perlin_grad(self.gradient(l + section_z + 1), local_x, local_y - 1.0, local_z - 1.0);
    let q =
      perlin_grad(self.gradient(n + section_z + 1), local_x - 1.0, local_y - 1.0, local_z - 1.0);
    let r = perlin_fade(local_x);
    let s = perlin_fade(fade_local_x);
    let t = perlin_fade(local_z);
    return lerp3(r, s, t, d, e, f, g, h, o, p, q);
  }
}

const GRADIENTS: [[i32; 3]; 16] = [
  [1, 1, 0],
  [-1, 1, 0],
  [1, -1, 0],
  [-1, -1, 0],
  [1, 0, 1],
  [-1, 0, 1],
  [1, 0, -1],
  [-1, 0, -1],
  [0, 1, 1],
  [0, -1, 1],
  [0, 1, -1],
  [0, -1, -1],
  [1, 1, 0],
  [0, -1, 1],
  [-1, 1, 0],
  [0, -1, -1],
];
fn perlin_grad(hash: i32, x: f64, y: f64, z: f64) -> f64 {
  let gradient = GRADIENTS[(hash & 0xf) as usize];
  gradient[0] as f64 * x + gradient[1] as f64 * y + gradient[2] as f64 * z
}
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

pub fn lerp(delta: f64, start: f64, end: f64) -> f64 { start + delta * (end - start) }

#[cfg(test)]
mod tests {
  use super::{super::super::rng::SimpleRng, *};
  use pretty_assertions::assert_eq;

  #[test]
  fn single_perlin_test() {
    let mut rng = SimpleRng::new(1);
    let mut perlin = Perlin::new(&mut rng);

    assert_similar(perlin.origin_x, 187.10481682004246);
    assert_eq!(
      perlin.kernel.iter().map(|num| format!("{num:08b}")).collect::<Vec<String>>(),
      [
        85_i8, 92, 42, 62, 65, -44, 73, 40, 50, -10, -94, 18, 80, -22, 20, -56, -50, -69, -108,
        -79, 82, 74, -105, 84, -115, 12, -90, 122, -85, 72, -65, -30, 59, 96, -35, -107, 93, 97,
        79, 63, 52, -117, 21, -40, -16, -5, -4, -62, 15, 104, 49, -1, -100, -31, -96, 86, -114,
        -103, -38, 83, 23, -41, -61, -95, -29, 56, 111, 39, -18, 24, 68, -49, -71, 123, 13, 51,
        -19, -84, -119, -76, -21, 45, 64, 102, -3, 118, -86, 31, 69, -67, -7, -104, -112, -52, -48,
        -23, -66, 61, 33, 3, -60, -77, 95, -26, 71, -126, 8, -6, 76, 103, -113, -125, 58, -14, 55,
        -118, -110, -43, 124, 29, -75, 37, -120, -13, 44, 94, 7, 81, -57, -70, -88, 77, 38, -116,
        -36, 88, -9, 1, -55, 35, 17, 116, 114, -92, -102, -98, -128, 70, -15, 66, 60, -124, 14,
        -93, 53, -91, 117, -74, -33, -58, -123, 5, 112, -82, 115, -53, -8, -80, 0, -73, 2, 99, 75,
        67, 4, 126, 30, 87, 109, -46, 22, 98, -64, -101, 127, 48, 36, -81, 78, 119, 32, -25, 100,
        121, -24, -99, -121, -12, 47, -32, -28, -106, 105, -89, 57, 25, 113, -51, -20, -11, -45,
        -37, 9, -87, -54, -27, -68, -34, -109, -97, -78, -39, 11, 28, 10, 26, 125, 16, 91, -72,
        110, -17, 101, -63, 120, -83, -2, 19, 54, -111, 90, 6, -59, 108, -127, 89, 41, 34, -42, 27,
        -122, 107, 43, 106, 46, -47
      ]
      .iter()
      .map(|num| format!("{num:08b}"))
      .collect::<Vec<String>>()
    );

    assert_similar(perlin.sample(0.0, 0.0, 0.0), 0.10709);
    assert_similar(perlin.sample(0.5, 0.0, 0.0), -0.2507);
  }

  fn assert_similar(actual: f64, expected: f64) {
    if (expected - actual).abs() > 0.0001 {
      panic!("Expected: {expected}, got: {actual}");
    }
  }
}
