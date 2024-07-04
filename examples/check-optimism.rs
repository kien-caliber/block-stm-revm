#![allow(missing_docs)]

use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{BlockId, BlockTransactionsKind};
use pevm::RpcStorage;
use reqwest::Url;
use revm::db::CacheDB;
use revm::primitives::SpecId;
use tokio::runtime::Runtime;

#[path = "../tests/common/mod.rs"]
pub mod common;

#[cfg(feature = "optimism")]
fn main() {
    use alloy_chains::Chain;
    use pevm::{get_block_spec, Storage, StorageWrapper};

    let rpc_url = Url::parse("https://mainnet.optimism.io").unwrap();
    let block_number: u64 = 121252980;
    let runtime = Runtime::new().unwrap();
    let provider = ProviderBuilder::new().on_http(rpc_url.clone());
    let block = runtime
        .block_on(provider.get_block(BlockId::number(block_number), BlockTransactionsKind::Full))
        .unwrap()
        .unwrap();
    let spec_id = get_block_spec(Chain::optimism_mainnet(), &block.header).unwrap();
    let pre_state_rpc_storage =
        RpcStorage::new(provider.clone(), spec_id, BlockId::number(block_number - 1));
    let pre_state_db = CacheDB::new(StorageWrapper(&pre_state_rpc_storage));
    let tx_results = common::test_execute_alloy(
        &pre_state_db,
        Chain::optimism_mainnet(),
        block.clone(),
        true,
    );

    let observed_storage = pre_state_rpc_storage;
    for tx_result in tx_results {
        observed_storage.update_cache_accounts(tx_result.state);
    }
    let observed_accounts = observed_storage.get_cache_accounts();
    let expected_storage = RpcStorage::new(provider, spec_id, BlockId::number(block_number));

    for (address, account) in observed_accounts {
        let expected_basic = expected_storage
            .basic(&address)
            .unwrap()
            .unwrap_or_default();
        assert_eq!(expected_basic, account.basic);
        for (storage_key, storage_value) in account.storage {
            let expected_value = expected_storage.storage(&address, &storage_key).unwrap();
            assert_eq!(expected_value, storage_value);
        }
    }
}
