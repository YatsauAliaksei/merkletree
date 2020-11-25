use criterion::{black_box, Criterion, criterion_group, criterion_main};
use merkletree::{Hash, MerkleTree};
use merkletree::hash::ShaHasher;
use rand::Rng;
use rand::seq::SliceRandom;
use std::time::Instant;

fn mt_update_benchmark(c: &mut Criterion) {
    let levels = 20;
    let mut tree = MerkleTree::new(levels, ShaHasher::default());
    println!("empty: {}", tree);

    let mut k = 0;
    let nodes_size = 1 << levels - 1;
    let mut time = std::time::Instant::now();
    while k < nodes_size {
        let hash = tree.generate_hash("hello".as_bytes());
        tree.add(hash);
        k += 1;
        if k % 100_000 == 0 {
            let time_100k_micros = time.elapsed().as_micros();
            println!("K: {}/{} - {}/{}us", k, nodes_size, time_100k_micros, time_100k_micros / 100_000);
            time = std::time::Instant::now();
        }
    }

    println!("Tree {}", tree);

    let test_size = 10;

    let mut random = rand::thread_rng();
    let mut indexes: Vec<u32> = (0..test_size).collect();
    indexes.shuffle(&mut random);

    let gen_hashes = prepare_hashes(test_size, &tree);

    let id = format!("MT: {}. Update: {}", tree.capacity(), test_size);

    c.bench_function(id.as_str(),
                     |b| b.iter(|| {
                         for i in &indexes {
                             match tree.update(*i, black_box(gen_hashes[*i as usize])) {
                                 Ok(v) => black_box(v),
                                 Err(e) => panic!("Update failed. {:?}", e)
                             };
                         }
                     }));
}

fn prepare_hashes(n: u32, tree: &MerkleTree) -> Vec<Hash> {
    let mut result: Vec<Hash> = vec![];

    for i in 0..n {
        let hash = tree.generate_hash(&i.to_be_bytes());
        result.push(hash);
    }

    result
}

criterion_group!(benches, mt_update_benchmark);
criterion_main!(benches);
