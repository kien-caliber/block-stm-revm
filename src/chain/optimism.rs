//! Optimism
#![allow(missing_docs)]
use std::collections::{BTreeMap, HashMap};

use alloy_chains::NamedChain;
use alloy_consensus::{Signed, TxEip1559, TxEip2930, TxEip4844, TxLegacy};
use alloy_primitives::{Bytes, B256, U256};
use alloy_rpc_types::{BlockTransactions, Header};
use op_alloy_consensus::{OpDepositReceipt, OpReceiptEnvelope, OpTxEnvelope, OpTxType, TxDeposit};
use op_alloy_network::eip2718::Encodable2718;
use op_alloy_rpc_types::Transaction;
use revm::{
    primitives::{BlockEnv, OptimismFields, SpecId, TxEnv},
    Handler,
};

use crate::{mv_memory::MvMemory, BuildIdentityHasher, MemoryLocation, PevmTxExecutionResult};

use super::{PevmChain, RewardPolicy};

/// Represents errors that can occur when parsing transactions
#[derive(Debug, Clone, PartialEq)]
pub enum OptimismTxEnvError {
    ConversionError(String),
    GasPriceError(OptimismGasPriceError),
    InvalidType(u8),
    MissingMaxFeePerGas,
    MissingSourceHash,
    OverflowedGasLimit,
    SerdeError(String),
    UnexpectedType(u8),
}

/// Convert [Transaction] to [OptimismFields]
pub(crate) fn get_optimism_fields(tx: &Transaction) -> Result<OptimismFields, OptimismTxEnvError> {
    let envelope_buf = {
        let tx_type = tx.inner.transaction_type.unwrap_or_default();
        let op_tx_type = OpTxType::try_from(tx_type)
            .map_err(|_err| OptimismTxEnvError::UnexpectedType(tx_type))?;
        let inner = tx.inner.clone();
        let tx_envelope = match op_tx_type {
            OpTxType::Legacy => Signed::<TxLegacy>::try_from(inner).map(OpTxEnvelope::from),
            OpTxType::Eip2930 => Signed::<TxEip2930>::try_from(inner).map(OpTxEnvelope::from),
            OpTxType::Eip1559 => Signed::<TxEip1559>::try_from(inner).map(OpTxEnvelope::from),
            OpTxType::Eip4844 => Signed::<TxEip4844>::try_from(inner).map(OpTxEnvelope::from),
            OpTxType::Deposit => {
                let tx_deposit = TxDeposit {
                    source_hash: tx
                        .source_hash
                        .ok_or(OptimismTxEnvError::MissingSourceHash)?,
                    from: tx.inner.from,
                    to: tx.inner.to.into(),
                    mint: tx.mint,
                    value: tx.inner.value,
                    gas_limit: tx.inner.gas,
                    is_system_transaction: tx.is_system_tx.unwrap_or_default(),
                    input: tx.inner.input.clone(),
                };
                Ok(OpTxEnvelope::from(tx_deposit))
            }
        }
        .map_err(|err| OptimismTxEnvError::ConversionError(err.to_string()))?;

        let mut envelope_buf = Vec::<u8>::new();
        tx_envelope.encode_2718(&mut envelope_buf);
        Bytes::from(envelope_buf)
    };

    Ok(OptimismFields {
        source_hash: tx.source_hash,
        mint: tx.mint,
        is_system_transaction: tx.is_system_tx,
        enveloped_tx: Some(envelope_buf),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptimismBlockSpecError {
    MissingBlockNumber,
    UnsupportedSpec,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptimismGasPriceError {
    InvalidType(u8),
    MissingGasPrice,
    MissingMaxFeePerGas,
}

// https://github.com/paradigmxyz/reth/blob/b4a1b733c93f7e262f1b774722670e08cdcb6276/crates/primitives/src/proofs.rs
fn encode_receipt_2718(
    spec_id: SpecId,
    tx: &Transaction,
    tx_result: &PevmTxExecutionResult,
) -> Bytes {
    let tx_type = tx.inner.transaction_type.unwrap_or_default();
    let op_tx_type = OpTxType::try_from(tx_type).unwrap();
    let receipt_envelope = match op_tx_type {
        OpTxType::Legacy => OpReceiptEnvelope::Legacy(tx_result.receipt.clone().with_bloom()),
        OpTxType::Eip2930 => OpReceiptEnvelope::Eip2930(tx_result.receipt.clone().with_bloom()),
        OpTxType::Eip1559 => OpReceiptEnvelope::Eip1559(tx_result.receipt.clone().with_bloom()),
        OpTxType::Eip4844 => OpReceiptEnvelope::Eip4844(tx_result.receipt.clone().with_bloom()),
        OpTxType::Deposit => {
            let account_maybe = tx_result
                .state
                .get(&tx.inner.from)
                .expect("Sender not found");
            let account = account_maybe.as_ref().expect("Sender not changed");
            let receipt = OpDepositReceipt {
                inner: tx_result.receipt.clone(),
                deposit_nonce: (spec_id >= SpecId::CANYON).then_some(account.nonce - 1),
                deposit_receipt_version: (spec_id >= SpecId::CANYON).then_some(1),
            };
            OpReceiptEnvelope::Deposit(receipt.with_bloom())
        }
    };

    let mut buffer = Vec::new();
    receipt_envelope.encode_2718(&mut buffer);
    Bytes::from(buffer)
}

/// Implementation of [PevmChain] for Ethereum
#[derive(Debug, Clone, PartialEq)]
pub struct PevmOptimism {
    id: u64,
}

impl PevmOptimism {
    pub fn mainnet() -> Self {
        PevmOptimism {
            id: NamedChain::Optimism.into(),
        }
    }
}

impl PevmChain for PevmOptimism {
    type Transaction = Transaction;
    type BlockSpecError = OptimismBlockSpecError;
    type GasPriceError = OptimismGasPriceError;
    type TxEnvError = OptimismTxEnvError;

    fn id(&self) -> u64 {
        self.id
    }

    fn get_block_spec(&self, header: &Header) -> Result<SpecId, Self::BlockSpecError> {
        if header.timestamp >= 1720627201 {
            Ok(SpecId::FJORD)
        } else if header.timestamp >= 1710374401 {
            Ok(SpecId::ECOTONE)
        } else if header.timestamp >= 1704992401 {
            Ok(SpecId::CANYON)
        } else if header.number >= 105235063 {
            Ok(SpecId::REGOLITH)
        } else {
            // TODO: revm does not support when L1Block is not available
            Err(OptimismBlockSpecError::UnsupportedSpec)
        }
    }

    fn get_gas_price(&self, tx: &Transaction) -> Result<U256, Self::GasPriceError> {
        let tx_type_raw = tx.inner.transaction_type.unwrap_or_default();
        let Ok(tx_type) = OpTxType::try_from(tx_type_raw) else {
            return Err(OptimismGasPriceError::InvalidType(tx_type_raw));
        };

        match tx_type {
            OpTxType::Legacy | OpTxType::Eip2930 => tx
                .inner
                .gas_price
                .map(U256::from)
                .ok_or(OptimismGasPriceError::MissingGasPrice),
            OpTxType::Eip1559 | OpTxType::Eip4844 => tx
                .inner
                .max_fee_per_gas
                .map(U256::from)
                .ok_or(OptimismGasPriceError::MissingMaxFeePerGas),
            OpTxType::Deposit => Ok(U256::ZERO),
        }
    }

    fn build_mv_memory(
        &self,
        hasher: &ahash::RandomState,
        block_env: &BlockEnv,
        txs: &[TxEnv],
    ) -> MvMemory {
        let beneficiary_location_hash = hasher.hash_one(MemoryLocation::Basic(block_env.coinbase));
        let l1_fee_recipient_location_hash = hasher.hash_one(revm::L1_FEE_RECIPIENT);
        let base_fee_recipient_location_hash = hasher.hash_one(revm::BASE_FEE_RECIPIENT);

        // TODO: Estimate more locations based on sender, to, etc.
        let mut estimated_locations = HashMap::with_hasher(BuildIdentityHasher::default());
        for (index, tx) in txs.iter().enumerate() {
            if tx.optimism.source_hash.is_none() {
                estimated_locations
                    .entry(beneficiary_location_hash)
                    .or_insert_with(|| Vec::with_capacity(txs.len()))
                    .push(index);
            } else {
                estimated_locations
                    .entry(l1_fee_recipient_location_hash)
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(index);
                estimated_locations
                    .entry(base_fee_recipient_location_hash)
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(index);
            }
        }

        MvMemory::new(
            txs.len(),
            estimated_locations,
            [
                block_env.coinbase,
                revm::L1_FEE_RECIPIENT,
                revm::BASE_FEE_RECIPIENT,
            ],
        )
    }

    fn get_handler<'a, EXT, DB: revm::Database>(
        &self,
        spec_id: SpecId,
        with_reward_beneficiary: bool,
    ) -> Handler<'a, revm::Context<EXT, DB>, EXT, DB> {
        Handler::optimism_with_spec(spec_id, with_reward_beneficiary)
    }

    fn get_reward_policy(&self, hasher: &ahash::RandomState) -> RewardPolicy {
        RewardPolicy::Optimism {
            l1_fee_recipient_location_hash: hasher
                .hash_one(MemoryLocation::Basic(revm::optimism::L1_FEE_RECIPIENT)),
            base_fee_vault_location_hash: hasher
                .hash_one(MemoryLocation::Basic(revm::optimism::BASE_FEE_RECIPIENT)),
        }
    }

    // Refer to section 4.3.2. Holistic Validity in the Ethereum Yellow Paper.
    // https://github.com/ethereum/go-ethereum/blob/master/cmd/era/main.go#L289
    fn calculate_receipt_root(
        &self,
        spec_id: SpecId,
        txs: &BlockTransactions<Transaction>,
        tx_results: &[PevmTxExecutionResult],
    ) -> B256 {
        let trie_entries: BTreeMap<_, _> = txs
            .txns()
            .zip(tx_results)
            .enumerate()
            .map(|(index, (tx, tx_result))| {
                let key_buffer = alloy_rlp::encode_fixed_size(&index).to_vec();
                let value_buffer = encode_receipt_2718(spec_id, tx, tx_result);
                (key_buffer, value_buffer)
            })
            .collect();

        let mut hash_builder = alloy_trie::HashBuilder::default();
        for (k, v) in trie_entries {
            hash_builder.add_leaf(alloy_trie::Nibbles::unpack(&k), &v);
        }
        hash_builder.root()
    }

    fn get_tx_env(&self, tx: Self::Transaction) -> Result<TxEnv, Self::TxEnvError> {
        Ok(TxEnv {
            optimism: get_optimism_fields(&tx)?,
            caller: tx.inner.from,
            gas_limit: tx
                .inner
                .gas
                .try_into()
                .map_err(|_| OptimismTxEnvError::OverflowedGasLimit)?,
            gas_price: self
                .get_gas_price(&tx)
                .map_err(OptimismTxEnvError::GasPriceError)?,
            gas_priority_fee: tx.inner.max_priority_fee_per_gas.map(U256::from),
            transact_to: tx.inner.to.into(),
            value: tx.inner.value,
            data: tx.inner.input,
            nonce: Some(tx.inner.nonce),
            chain_id: tx.inner.chain_id,
            access_list: tx.inner.access_list.unwrap_or_default().into(),
            blob_hashes: tx.inner.blob_versioned_hashes.unwrap_or_default(),
            max_fee_per_blob_gas: tx.inner.max_fee_per_blob_gas.map(U256::from),
            authorization_list: None, // TODO: Support in the upcoming hardfork
        })
    }
}
