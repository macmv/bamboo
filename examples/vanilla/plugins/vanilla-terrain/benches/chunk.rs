use criterion::{criterion_group, criterion_main, Criterion};
use vanilla_terrain::{
  common::{
    chunk::{paletted, Chunk},
    math::{ChunkPos, RelPos},
  },
  vanilla,
};

mod perf;

fn generate_chunk(c: &mut Criterion) {
  let mut pos = ChunkPos::new(0, 0);
  c.bench_function("chunk_generate", |b| {
    b.iter_batched(
      || Chunk::new(8),
      |mut chunk| {
        pos = ChunkPos::new(pos.x() + 1, pos.z());
        vanilla::generate_chunk(&mut chunk, pos)
      },
      criterion::BatchSize::SmallInput,
    )
  });
  let mut pos = ChunkPos::new(0, 0);
  let mut out = vec![0; 1024 * 64];
  c.bench_function("chunk_serialize", |b| {
    b.iter_batched(
      || {
        let mut chunk = criterion::black_box(Chunk::<paletted::Section>::new(8));
        for y in 0..256 {
          chunk.set_block(RelPos::new(0, y, 0), 1).unwrap();
        }
        chunk
      },
      |chunk| {
        use bb_plugin::transfer::MessageWriter;

        let mut sections = vec![];
        for section in chunk.sections().flatten() {
          sections.push(section);
        }

        let mut writer = MessageWriter::new(&mut out);
        writer.write(&sections).unwrap();
        use_val(criterion::black_box(&out));
      },
      criterion::BatchSize::SmallInput,
    )
  });
}

#[inline(never)]
fn use_val<T>(val: T) {}

criterion_group! {
  name = chunk;
  // This can be any expression that returns a `Criterion` object.
  config = Criterion::default().with_profiler(perf::FlamegraphProfiler::new(100));
  targets = generate_chunk
}
criterion_main!(chunk);
