use alloy_chains::Chain;
use alloy_primitives::{Bloom, Bytes, B256};
use alloy_rpc_types::Receipt;
use alloy_rpc_types::{Block, BlockTransactions, Transaction};
use pevm::{EnvelopeBuilder, EvmAccount, PevmResult, PevmTxExecutionResult, Storage};
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

#[cfg(feature = "optimism")]
const DEPOSIT_TX_TYPE_ID: u8 = 126;

fn encode_receipt_2718(
    chain: Chain,
    tx_type: u8,
    receipt: Receipt,
    deposit_nonce: Option<u64>,
) -> Bytes {
    let mut eb = EnvelopeBuilder::with_capacity(6);
    eb.push(&receipt.status);
    eb.push(&receipt.cumulative_gas_used);
    eb.push(&receipt.bloom_slow());
    eb.push(&receipt.logs);
    #[cfg(feature = "optimism")]
    if chain.is_optimism() && tx_type == DEPOSIT_TX_TYPE_ID {
        eb.push(&deposit_nonce.expect("deposit_nonce not provided"));
        eb.push(&1u64); // deposit_receipt_version
    }
    eb.to_bytes_with_header(if tx_type == 0 { None } else { Some(tx_type) })
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

            let mut deposit_nonce = None;
            #[cfg(feature = "optimism")]
            if chain.is_optimism() && tx_type == DEPOSIT_TX_TYPE_ID {
                let account_maybe = result.state.get(&tx.from).expect("Sender not found");
                let account = account_maybe.as_ref().expect("Sender not changed");
                deposit_nonce = Some(account.basic.nonce - 1);
            }

            let value_buffer =
                encode_receipt_2718(chain, tx_type, result.receipt.clone(), deposit_nonce);
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
