#![cfg(feature = "optimism")]

use alloy_primitives::Bloom;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{BlockId, BlockTransactionsKind};
use op_alloy_network::Optimism;
use pevm::{
    chain::{PevmChain, PevmOptimism},
    EvmAccount, Pevm, RpcStorage, Storage,
};
use reqwest::Url;
use revm::primitives::SpecId;
use tokio::runtime::Runtime;

pub mod common;

#[test]
fn optimism_blocks_from_rpc() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = match std::env::var("OPTIMISM_RPC_URL") {
        // The empty check is for GitHub Actions where the variable is set with an empty string when unset!?
        Ok(value) if !value.is_empty() => value.parse()?,
        _ => Url::parse("https://mainnet.optimism.io")?,
    };

    for block_number in [
        // Here are random 8 blocks. The latter 6 have been commented because the CI is too slow.
        111435579,
        112003352,
        // 114470424, 114673497, 118446712, 118931697, 121413120, 123129762,
    ] {
        dbg!(block_number);
        let runtime = Runtime::new()?;
        let provider = ProviderBuilder::<_, _, Optimism>::default().on_http(rpc_url.clone());
        let block_opt = runtime.block_on(provider.get_block(
            BlockId::number(block_number), //
            BlockTransactionsKind::Full,
        ))?;
        let block = block_opt.ok_or("missing block")?;
        let chain = PevmOptimism::mainnet();
        let spec_id = chain.get_block_spec(&block.header).unwrap();
        let pre_state_rpc_storage =
            RpcStorage::new(provider.clone(), spec_id, BlockId::number(block_number - 1));

        let concurrency_level =
            std::thread::available_parallelism().unwrap_or(std::num::NonZeroUsize::MIN);

        let mut pevm = Pevm::default();
        let sequential_result = pevm.execute(
            &pre_state_rpc_storage,
            &chain,
            block.clone(),
            concurrency_level,
            true,
        );
        let parallel_result = pevm.execute(
            &pre_state_rpc_storage,
            &chain,
            block.clone(),
            concurrency_level,
            false,
        );

        assert_eq!(&sequential_result, &parallel_result);
        let tx_results = sequential_result.unwrap();

        // We can only calculate the receipts root from Byzantium.
        // Before EIP-658 (https://eips.ethereum.org/EIPS/eip-658), the
        // receipt root is calculated with the post transaction state root,
        // which we doesn't have in these tests.
        if spec_id >= SpecId::BYZANTIUM {
            assert_eq!(
                block.header.receipts_root,
                chain.calculate_receipt_root(spec_id, &block.transactions, &tx_results)
            );
        }

        assert_eq!(
            block.header.logs_bloom,
            tx_results
                .iter()
                .map(|tx| tx.receipt.bloom_slow())
                .fold(Bloom::default(), |acc, bloom| acc.bit_or(bloom))
        );

        assert_eq!(
            block.header.gas_used,
            tx_results
                .iter()
                .last()
                .map(|result| result.receipt.cumulative_gas_used)
                .unwrap_or_default()
        );

        let mut observed_accounts = pre_state_rpc_storage.get_cache_accounts();
        for tx_result in tx_results {
            for (address, account) in tx_result.state {
                let target_account = observed_accounts.entry(address).or_default();
                if let Some(account) = account {
                    target_account.balance = account.balance;
                    target_account.nonce = account.nonce;
                    target_account.storage.extend(account.storage);
                } else {
                    *target_account = EvmAccount::default();
                }
            }
        }

        let expected_storage = RpcStorage::new(provider, spec_id, BlockId::number(block_number));

        for (address, account) in observed_accounts {
            let expected_basic = expected_storage
                .basic(&address)
                .unwrap()
                .unwrap_or_default();
            assert_eq!(expected_basic.balance, account.balance,);
            assert_eq!(expected_basic.nonce, account.nonce,);
            for (storage_key, storage_value) in account.storage {
                let expected_value = expected_storage.storage(&address, &storage_key).unwrap();
                assert_eq!(expected_value, storage_value);
            }
        }
    }

    Ok(())
}
