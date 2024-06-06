// TODO: Move this into `tests/ethereum`.
// TODO: `tokio::test`?

use std::fs::{self, File};

use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::BlockId;
use pevm::RpcStorage;
use reqwest::Url;
use revm::db::CacheDB;
use tokio::runtime::Runtime;

pub mod common;

#[test]
fn mainnet_blocks_from_rpc() {
    let rpc_url = match std::env::var("RPC_URL") {
        Ok(value) if !value.is_empty() => value.parse().unwrap(),
        _ => Url::parse("https://eth.llamarpc.com").unwrap(),
    };

    // First block under 50 transactions of each EVM-spec-changing fork
    for block_number in [
        // 19933597,
        // 1150000, 11814555, 12243999, 12244000, 12300570, 12520364, 12522062, 12964999, 12965000,
        // 13217637, 13287210, 14029313, 14334629, 14383540, 14396881, 15199017,
        15537393,
        // 15537394, 15538827, 16146267, 17034869, 17034870, 17666333, 19426586, 19426587, 19638737,
        // 19807137, 19917570, 19923400, 19929064, 19932148, 19932703, 19932810, 19933122, 19933597,
        // 19933612, 19934116, 2179522, 2462997,
        2641321,
        // 2674998, 2675000, 4330482, 4369999, 4370000, 46147, 5891667, 7279999, 7280000, 8889776,
        // 9068998, 9069000,
        // 930196,
        // 46147, // FRONTIER
        // 1150000, // HOMESTEAD
        // TODO: Enable these when CI is less flaky.
        // 2463002,  // TANGERINE
        // 2675000,  // SPURIOUS_DRAGON
        // 4370003,  // BYZANTIUM
        // 7280003,  // PETERSBURG
        // 9069001,  // ISTANBUL
        // 12244002, // BERLIN
        // 12965034, // LONDON
        // 15537395, // MERGE
        // 17035010, // SHANGHAI
        // 19426587, // CANCUN
    ] {
        println!("{:?}", block_number);
        let runtime = Runtime::new().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url.clone());
        let block = runtime
            .block_on(provider.get_block(BlockId::number(block_number), true))
            .unwrap()
            .unwrap();
        let rpc_storage = RpcStorage::new(provider, BlockId::number(block_number - 1));
        let db = CacheDB::new(&rpc_storage);
        common::test_execute_alloy(db.clone(), block.clone(), None, true, true, true);

        // Snapshot blocks (for benchmark)
        // TODO: Port to a dedicated CLI instead?
        // TODO: Binary formats to save disk?
        if std::env::var("SNAPSHOT_BLOCKS") == Ok("1".to_string()) {
            let dir = format!("blocks/{block_number}");
            fs::create_dir_all(dir.clone()).unwrap();

            let block_hashes = &rpc_storage.get_cache_block_hashes();
            if !block_hashes.is_empty() {
                let file_block = File::create(format!("{dir}/block.json")).unwrap();
                serde_json::to_writer(file_block, &block).unwrap();
                let file_state = File::create(format!("{dir}/state_for_execution.json")).unwrap();
                serde_json::to_writer(file_state, &rpc_storage.get_cache()).unwrap();

                let file = File::create(format!("{dir}/block_hashes.json")).unwrap();
                serde_json::to_writer(file, &block_hashes).unwrap();
            }
        }
    }
}

// #[test]
// fn mainnet_blocks_from_disk() {
//     common::for_each_block_from_disk(|block, state| {
//         println!("{:?}", block.header.number);
//         // Run several times to try catching a race condition if there is any.
//         // 1000~2000 is a better choice for local testing after major changes.
//         for _ in 0..3 {
//             common::test_execute_alloy(
//                 common::build_in_mem(state.clone()),
//                 block.clone(),
//                 None,
//                 true,
//                 true,
//                 true,
//             )
//         }
//     });
// }
