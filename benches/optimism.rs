//! Benchmark Optimism
#![cfg(feature = "optimism")]
#![allow(missing_docs)]

use std::{
    collections::BTreeMap, fs::File, io::BufReader, num::NonZeroUsize, path::PathBuf, thread,
};

use alloy_primitives::Address;
use alloy_rpc_types::Block;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use op_alloy_rpc_types::Transaction;
use pevm::{chain::PevmOptimism, Bytecodes, EvmAccount, InMemoryStorage, Pevm};

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

struct Sample {
    block_number: u64,
    path_to_block: PathBuf,
    path_to_pre_state: PathBuf,
}

impl Sample {
    fn load_block(&self) -> Block<Transaction> {
        serde_json::from_reader(BufReader::new(File::open(&self.path_to_block).unwrap())).unwrap()
    }

    fn load_pre_state(&self) -> BTreeMap<Address, EvmAccount> {
        serde_json::from_reader(BufReader::new(File::open(&self.path_to_pre_state).unwrap()))
            .unwrap()
    }
}

struct SampleSet {
    path_to_bytecodes: PathBuf,
    samples: Vec<Sample>,
}

impl Default for SampleSet {
    fn default() -> Self {
        Self {
            path_to_bytecodes: PathBuf::from("target/benches/optimism/bytecodes.bincode"),
            samples: vec![
                Sample {
                    block_number: 124830410,
                    path_to_block: PathBuf::from("target/benches/optimism/124830410/block.json"),
                    path_to_pre_state: PathBuf::from(
                        "target/benches/optimism/124830410/pre_state.json",
                    ),
                },
                Sample {
                    block_number: 124828225,
                    path_to_block: PathBuf::from("target/benches/optimism/124828225/block.json"),
                    path_to_pre_state: PathBuf::from(
                        "target/benches/optimism/124828225/pre_state.json",
                    ),
                },
            ],
        }
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let chain = PevmOptimism::mainnet();
    let concurrency_level = thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        // This max should be tuned to the running machine,
        // ideally also per block depending on the number of
        // transactions, gas usage, etc. ARM machines seem to
        // go higher thanks to their low thread overheads.
        .min(
            NonZeroUsize::new(
                #[cfg(target_arch = "aarch64")]
                12,
                #[cfg(not(target_arch = "aarch64"))]
                8,
            )
            .unwrap(),
        );
    let mut pevm = Pevm::default();

    let sample_set = SampleSet::default();
    let bytecodes: Bytecodes = bincode::deserialize_from(BufReader::new(
        File::open(sample_set.path_to_bytecodes).unwrap(),
    ))
    .unwrap();

    for sample in sample_set.samples {
        dbg!(sample.block_number);
        let block = sample.load_block();
        let pre_state = sample.load_pre_state();
        let storage = InMemoryStorage::new(pre_state, Some(&bytecodes), []);

        let mut group = c.benchmark_group(format!(
            "Block {} ({} txs, {} gas)",
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
                    black_box(concurrency_level),
                    black_box(true),
                )
            })
        });

        group.bench_function("Parallel", |b| {
            b.iter(|| {
                pevm.execute(
                    black_box(&storage),
                    black_box(&chain),
                    black_box(block.clone()),
                    black_box(concurrency_level),
                    black_box(false),
                )
            })
        });

        group.finish();
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
