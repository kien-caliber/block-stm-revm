//! Benchemark mainnet blocks with needed state loaded in memory.

// TODO: More fancy benchmarks & plots.

#![allow(missing_docs)]

use std::num::NonZeroUsize;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pevm::chain::PevmEthereum;

// Better project structure
#[path = "../tests/common/mod.rs"]
pub mod common;

#[global_allocator]
static GLOBAL: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

pub fn criterion_benchmark(c: &mut Criterion) {
    let chain = PevmEthereum::mainnet();

    common::for_each_block_from_disk(|block, storage| {
        let mut group = c.benchmark_group(format!(
            "Block {}({} txs, {} gas)",
            block.header.number.unwrap(),
            block.transactions.len(),
            block.header.gas_used
        ));
        group.warm_up_time(std::time::Duration::from_millis(100));
        group.sample_size(10);
        group.measurement_time(std::time::Duration::from_secs(1));
        group.sampling_mode(criterion::SamplingMode::Flat);

        group.bench_function("S", |b| {
            b.iter(|| {
                pevm::execute(
                    black_box(&storage),
                    black_box(&chain),
                    black_box(block.clone()),
                    black_box(NonZeroUsize::MIN),
                    black_box(true),
                )
            })
        });

        for concurrency_level in 4..=16 {
            group.bench_function(format!("P{concurrency_level}"), |b| {
                b.iter(|| {
                    pevm::execute(
                        black_box(&storage),
                        black_box(&chain),
                        black_box(block.clone()),
                        black_box(NonZeroUsize::new(concurrency_level).unwrap()),
                        black_box(false),
                    )
                })
            });
        }

        group.finish();
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
