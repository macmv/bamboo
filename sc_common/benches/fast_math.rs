use common::math::FastMath;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

pub fn fast_cos(c: &mut Criterion) {
  c.bench_function("fast cos f64", |b| {
    let mut i = 0.0_f64;
    b.iter(|| {
      black_box(i.fast_cos());
      i += 0.1;
    });
  });
}
pub fn cos(c: &mut Criterion) {
  c.bench_function("cos f64", |b| {
    let mut i = 0.0_f64;
    b.iter(|| {
      black_box(i.cos());
      i += 0.1;
    });
  });
}

criterion_group! {
  name = benches;
  config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
  targets = fast_cos, cos
}
criterion_main!(benches);
