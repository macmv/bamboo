mod double;
mod octave;
mod perlin;

pub use double::Double;
pub use octave::Octave;
pub use perlin::Perlin;

pub type DoublePerlin = Double<Octave<Perlin>>;
pub type OctavePerlin = Octave<Perlin>;

pub trait Noise {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64;
}
