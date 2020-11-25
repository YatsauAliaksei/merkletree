use criterion::{black_box, Criterion, criterion_group, criterion_main};
use rand::Rng;
use rand::seq::SliceRandom;

use merkletree::{Hash, MerkleTree};
use merkletree::hash::ShaHasher;

fn mt_create_benchmark(c: &mut Criterion) {
    let levels = 20;

    let id = format!("MT: {}. Tree creation.", levels);

    c.bench_function(id.as_str(),
                     |b| b.iter(|| {
                         let v = MerkleTree::new(levels, ShaHasher::default());
                         black_box(v);
                     }));
}

criterion_group!(benches, mt_create_benchmark);
criterion_main!(benches);
