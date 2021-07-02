use common::{
  chunk::{fixed, paletted, Section},
  math::Pos,
};
use criterion::{criterion_group, criterion_main, Criterion};

pub fn paletted(c: &mut Criterion) {
  c.bench_function("paletted fill auto", |b| {
    let mut s = paletted::Section::new();
    let mut i = 0_u8;
    b.iter(move || {
      s.fill(Pos::new(0, 0, 0), Pos::new(15, 15, 15), i.into()).unwrap();
      i += 1;
    })
  });
  c.bench_function("paletted fill manual", |b| {
    let mut s = paletted::Section::new();
    let mut i = 0_u8;
    b.iter(move || {
      for y in 0..16 {
        for z in 0..16 {
          for x in 0..16 {
            s.set_block(Pos::new(x, y, z), i.into()).unwrap();
          }
        }
      }
      i += 1;
    })
  });
}
pub fn fixed(c: &mut Criterion) {
  c.bench_function("fixed fill auto", |b| {
    let mut s = fixed::Section::new();
    let mut i = 0_u8;
    b.iter(move || {
      s.fill(Pos::new(0, 0, 0), Pos::new(15, 15, 15), i.into()).unwrap();
      i += 1;
    })
  });
  c.bench_function("fixed fill manual", |b| {
    let mut s = fixed::Section::new();
    let mut i = 0_u8;
    b.iter(move || {
      for y in 0..16 {
        for z in 0..16 {
          for x in 0..16 {
            s.set_block(Pos::new(x, y, z), i.into()).unwrap();
          }
        }
      }
      i += 1;
    })
  });
}

criterion_group!(benches, paletted, fixed);
criterion_main!(benches);
