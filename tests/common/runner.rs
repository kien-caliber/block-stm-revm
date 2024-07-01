use alloy_chains::Chain;
use alloy_primitives::{Bloom, B256};
use alloy_rlp::Encodable;
use alloy_rpc_types::{Block, BlockTransactions, Transaction};
use pevm::{EvmAccount, PevmResult, PevmTxExecutionResult, Storage};
use revm::primitives::{alloy_primitives::U160, Address, BlockEnv, SpecId, TxEnv, U256};
use std::{collections::BTreeMap, num::NonZeroUsize, thread};

// Mock an account from an integer index that is used as the address.
// Useful for mock iterations.
pub fn mock_account(idx: usize) -> (Address, EvmAccount) {
    let address = Address::from(U160::from(idx));
    (
        address,
        // Filling half full accounts to have enough tokens for tests without worrying about
        // the corner case of balance not going beyond `U256::MAX`.
        EvmAccount::with_balance(U256::MAX.div_ceil(U256::from(2))),
    )
}

pub fn assert_execution_result(sequential_result: &PevmResult, parallel_result: &PevmResult) {
    assert_eq!(sequential_result, parallel_result);
}

// Execute an REVM block sequentially & with PEVM and assert that
// the execution results match.
pub fn test_execute_revm<S: Storage + Clone + Send + Sync>(storage: S, txs: Vec<TxEnv>) {
    let concurrency_level = thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
    assert_execution_result(
        &pevm::execute_revm_sequential(
            storage.clone(),
            Chain::mainnet(),
            SpecId::LATEST,
            BlockEnv::default(),
            txs.clone(),
        ),
        &pevm::execute_revm(
            storage,
            Chain::mainnet(),
            SpecId::LATEST,
            BlockEnv::default(),
            txs,
            concurrency_level,
        ),
    );
}

// Refer to section 4.3.2. Holistic Validity in the Ethereum Yellow Paper.
// https://specs.optimism.io/protocol/deposits.html#deposit-receipt
// https://github.com/ethereum/go-ethereum/blob/master/cmd/era/main.go#L289
// https://github.com/risechain/rise-reth/blob/d611f11a07fc7192595f58c5effcb3199aacbf61/crates/primitives/src/receipt.rs#L487-L503
// https://github.com/risechain/rise-reth/blob/6a104cc17461bac28164f3c2f08e7e1889708ab6/crates/revm/src/optimism/processor.rs#L133
fn calculate_receipt_root(
    chain: Chain,
    txs: &BlockTransactions<Transaction>,
    tx_results: &[PevmTxExecutionResult],
) -> B256 {
    let trie_entries: BTreeMap<_, _> = Iterator::zip(txs.txns(), tx_results)
        .enumerate()
        .map(|(index, (tx, result))| {
            let tx_type = tx.transaction_type.unwrap_or_default();

            let mut buf = Vec::new();
            result.receipt.status.encode(&mut buf);
            result.receipt.cumulative_gas_used.encode(&mut buf);
            result.receipt.bloom_slow().encode(&mut buf);
            result.receipt.logs.encode(&mut buf);

            if chain.is_optimism() && tx_type == 126 {
                let account_maybe = result.state.get(&tx.from).expect("Sender not found");
                let account = account_maybe.as_ref().expect("Sender not changed");
                let deposit_nonce = account.basic.nonce - 1;
                deposit_nonce.encode(&mut buf);
                let deposit_receipt_version: u64 = 1;
                deposit_receipt_version.encode(&mut buf);
            }

            let mut value_buffer = Vec::new();
            if tx_type != 0 {
                tx_type.encode(&mut value_buffer);
            };
            let rlp_head = alloy_rlp::Header {
                list: true,
                payload_length: buf.len(),
            };
            rlp_head.encode(&mut value_buffer);
            value_buffer.append(&mut buf);

            let key_buffer = alloy_rlp::encode_fixed_size(&index);
            let key_nibbles = alloy_trie::Nibbles::unpack(key_buffer);
            (key_nibbles, value_buffer)
        })
        .collect();

    let mut hash_builder = alloy_trie::HashBuilder::default();
    for (k, v) in trie_entries {
        hash_builder.add_leaf(k, &v);
    }
    hash_builder.root()
}

// Execute an Alloy block sequentially & with PEVM and assert that
// the execution results match.
pub fn test_execute_alloy<S: Storage + Clone + Send + Sync>(
    storage: S,
    chain: Chain,
    block: Block,
    must_match_block_header: bool,
) -> Vec<PevmTxExecutionResult> {
    let concurrency_level = thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
    let sequential_result = pevm::execute(
        storage.clone(),
        chain,
        block.clone(),
        concurrency_level,
        true,
    );
    let parallel_result = pevm::execute(storage, chain, block.clone(), concurrency_level, false);
    assert_execution_result(&sequential_result, &parallel_result);
    let tx_results = sequential_result.unwrap();

    if must_match_block_header {
        for (index, tx_result) in tx_results.iter().enumerate() {
            println!("{:02}: {:?}", index, tx_result.receipt);
        }

        // We can only calculate the receipts root from Byzantium.
        // Before EIP-658 (https://eips.ethereum.org/EIPS/eip-658), the
        // receipt root is calculated with the post transaction state root,
        // which we doesn't have in these tests.
        if block.header.number.unwrap() >= 4370000 {
            assert_eq!(
                block.header.receipts_root,
                calculate_receipt_root(chain, &block.transactions, &tx_results)
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
    }

    tx_results
}
