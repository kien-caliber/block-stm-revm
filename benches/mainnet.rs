//! Benchmark mainnet blocks with needed state loaded in memory.

// TODO: More fancy benchmarks & plots.

#![allow(missing_docs)]

use std::time::{Duration, Instant};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pevm::{chain::PevmEthereum, strategy::PevmStrategy, Pevm};

// Better project structure
#[path = "../tests/common/mod.rs"]
pub mod common;

// [rpmalloc] is generally better but can crash on AWS Graviton.
#[cfg(target_arch = "aarch64")]
#[global_allocator]
static GLOBAL: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;
#[cfg(not(target_arch = "aarch64"))]
#[global_allocator]
static GLOBAL: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

pub fn criterion_benchmark(c: &mut Criterion) {
    let chain = PevmEthereum::mainnet();
    let mut pevm = Pevm::default();

    common::for_each_block_from_disk(|block, storage| {
        let mut group = c.benchmark_group(format!(
            "Block {} {} {}",
            block.header.number,
            block.transactions.len(),
            block.header.gas_used
        ));
        group.bench_function("Sequential", |b| {
            b.iter(|| {
                pevm.execute(
                    black_box(&storage),
                    black_box(&chain),
                    black_box(block.clone()),
                    black_box(PevmStrategy::sequential()),
                )
            })
        });

        let mut attempts = Vec::new();

        group.bench_function("Parallel", |b| {
            b.iter_custom(|iters| {
                let mut total_time = Duration::ZERO;
                for _ in 0..iters {
                    let started_at = Instant::now();
                    let _ = pevm.execute(
                        black_box(&storage),
                        black_box(&chain),
                        black_box(block.clone()),
                        black_box(PevmStrategy::auto(
                            block.transactions.len(),
                            block.header.gas_used,
                        )),
                    );
                    let finished_at = Instant::now();
                    let svg_content = pevm.to_svg(started_at, finished_at);
                    attempts.push((finished_at.duration_since(started_at), svg_content));
                    total_time += finished_at.duration_since(started_at);
                }
                total_time
            })
        });

        attempts.sort_by_key(|(duration, _)| *duration);
        if let Some((_, svg_content)) = attempts.get(attempts.len() / 2) {
            let file_path =
                std::path::PathBuf::from(format!("target/svgs/{}.svg", block.header.number));
            std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
            let mut file = std::fs::File::create(&file_path).unwrap();
            std::io::Write::write_all(&mut file, svg_content.as_bytes()).unwrap();
        }

        group.finish();
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
