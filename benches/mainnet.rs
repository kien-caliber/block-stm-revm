//! Benchemark mainnet blocks with needed state loaded in memory.

// TODO: More fancy benchmarks & plots.

#![allow(missing_docs)]

use std::{num::NonZeroUsize, thread};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pevm::chain::PevmEthereum;

// Better project structure
#[path = "../tests/common/mod.rs"]
pub mod common;

#[global_allocator]
static GLOBAL: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

pub fn criterion_benchmark(c: &mut Criterion) {
    let chain = PevmEthereum::mainnet();
    let concurrency_level = thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        // 8 seems to be the sweet max for Ethereum blocks. Any more
        // will yield many overheads and hurt execution on (small) blocks
        // with many dependencies.
        .min(NonZeroUsize::new(8).unwrap());

    common::for_each_block_from_disk(|block, in_memory_storage, on_disk_storage| {
        let mut group = c.benchmark_group(format!(
            "Block {}({} txs, {} gas)",
            block.header.number.unwrap(),
            block.transactions.len(),
            block.header.gas_used
        ));
        group.bench_function("Sequential (in memory)", |b| {
            b.iter(|| {
                pevm::execute(
                    black_box(&in_memory_storage),
                    black_box(&chain),
                    black_box(block.clone()),
                    black_box(concurrency_level),
                    black_box(true),
                )
            })
        });
        group.bench_function("Parallel (in memory)", |b| {
            b.iter(|| {
                pevm::execute(
                    black_box(&in_memory_storage),
                    black_box(&chain),
                    black_box(block.clone()),
                    black_box(concurrency_level),
                    black_box(false),
                )
            })
        });
        group.bench_function("Sequential (on_disk)", |b| {
            b.iter(|| {
                pevm::execute(
                    black_box(&on_disk_storage),
                    black_box(&chain),
                    black_box(block.clone()),
                    black_box(concurrency_level),
                    black_box(true),
                )
            })
        });
        on_disk_storage.clear_cache();

        group.bench_function("Parallel (on_disk)", |b| {
            b.iter(|| {
                pevm::execute(
                    black_box(&on_disk_storage),
                    black_box(&chain),
                    black_box(block.clone()),
                    black_box(concurrency_level),
                    black_box(false),
                )
            })
        });
        on_disk_storage.clear_cache();

        group.finish();
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
