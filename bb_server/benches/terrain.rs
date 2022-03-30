use criterion::{criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use bb_common::math::ChunkPos;
use bb_server::{
  block,
  world::{chunk::MultiChunk, gen::WorldGen},
};
use std::sync::Arc;

pub fn generate_chunk(c: &mut Criterion) {
  c.bench_function("single chunk", |b| {
    let mut x = 0_i32;
    let mut z = 0_i32;
    let mut g = WorldGen::new();
    let types = Arc::new(block::TypeConverter::new());
    let mut c = MultiChunk::new(types);
    b.iter(move || {
      g.generate(ChunkPos::new(x, z), &mut c);
      x += 1;
      // We have a default view distance of 10, so a 21x21 region is the typical
      // region we are generating.
      if x > 21 {
        x = 0;
        z += 1;
      }
    })
  });
}

criterion_group! {
  name = benches;
  config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Protobuf));
  targets = generate_chunk
}
criterion_main!(benches);
