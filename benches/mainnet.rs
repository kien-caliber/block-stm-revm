//! Benchmark mainnet blocks with needed state loaded in memory.

// TODO: More fancy benchmarks & plots.

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pevm::{chain::PevmEthereum, ParallelParams, Pevm, PevmStrategy};

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
            "BLK {} {} {}",
            block.header.number,
            block.transactions.len(),
            block.header.gas_used
        ));

        group.sampling_mode(criterion::SamplingMode::Flat);
        group.sample_size(10);
        group.warm_up_time(std::time::Duration::from_millis(250));

        group.bench_function("S", |b| {
            b.iter(|| {
                assert!(pevm
                    .execute(
                        black_box(&storage),
                        black_box(&chain),
                        black_box(block.clone()),
                        black_box(PevmStrategy::sequential()),
                    )
                    .is_ok());
            })
        });
        for r in [6, 8, 10] {
            for p in [6, 8, 10] {
                for n in [16, 24, 32] {
                    group.bench_function(format!("P_{:02}_{:02}_{:02}", r, p, n), |b| {
                        b.iter(|| {
                            assert!(pevm
                                .execute(
                                    black_box(&storage),
                                    black_box(&chain),
                                    black_box(block.clone()),
                                    black_box(PevmStrategy::Parallel(ParallelParams {
                                        num_threads_for_regular_txs: r,
                                        num_threads_for_priority_txs: p,
                                        max_num_priority_txs: n
                                    })),
                                )
                                .is_ok());
                        })
                    });
                }
            }
        }

        group.finish();
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
