use criterion::{criterion_group, criterion_main, Criterion};
use vanilla_terrain::{
  common::{chunk::Chunk, math::ChunkPos},
  vanilla,
};

mod perf;

fn generate_chunk(c: &mut Criterion) {
  let mut pos = ChunkPos::new(0, 0);
  c.bench_function("chunk", |b| {
    b.iter_batched(
      || Chunk::new(8),
      |mut chunk| {
        pos = ChunkPos::new(pos.x() + 1, pos.z());
        vanilla::generate_chunk(&mut chunk, pos)
      },
      criterion::BatchSize::SmallInput,
    )
  });
}

criterion_group! {
  name = chunk;
  // This can be any expression that returns a `Criterion` object.
  config = Criterion::default().with_profiler(perf::FlamegraphProfiler::new(100));
  targets = generate_chunk
}
criterion_main!(chunk);
