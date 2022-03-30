use criterion::{criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use bb_common::{
  chunk::{fixed, paletted, Section},
  math::Pos,
};

pub fn paletted(c: &mut Criterion) {
  // # Test results
  //
  // Initial results (with libtest):
  // Opt level:        0        |       1      |      2     |
  // Fill manual: ~2,000,000 ns  ~1,200,000 ns   ~100,000ns
  // Fill:        ~9,000 ns      ~5,000 ns       ~300ns
  //
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
  // # Test results
  //
  // Original results (using libtest):
  // Optlevel:          0    |     1     |    2
  // Fill:        ~200,000ns   ~78,000ns   ~5,000ns
  // Fill manual: ~200,000ns   ~76,000ns   ~7,500ns
  //
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

criterion_group! {
  name = benches;
  config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
  targets = paletted, fixed
}
criterion_main!(benches);
