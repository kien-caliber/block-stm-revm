// TODO: Support custom chains like OP & RISE
// Ideally REVM & Alloy would provide all these.

use alloy_chains::Chain;
use alloy_consensus::TxEnvelope;
use alloy_primitives::{Bytes, B256, U128};
use alloy_provider::network::eip2718::Encodable2718;
use alloy_rpc_types::{Header, Transaction};
use revm::primitives::{
    BlobExcessGasAndPrice, BlockEnv, OptimismFields, SpecId, TransactTo, TxEnv, U256,
};

/// Get the REVM spec id of an Alloy block.
// Currently hardcoding Ethereum hardforks from these reference:
// https://github.com/paradigmxyz/reth/blob/4fa627736681289ba899b38f1c7a97d9fcf33dc6/crates/primitives/src/revm/config.rs#L33-L78
// https://github.com/paradigmxyz/reth/blob/4fa627736681289ba899b38f1c7a97d9fcf33dc6/crates/primitives/src/chain/spec.rs#L44-L68
// TODO: Better error handling & properly test this.
pub fn get_block_spec(chain: Chain, header: &Header) -> Option<SpecId> {
    #[cfg(feature = "optimism")]
    if chain.is_optimism() {
        // TODO: complete this function
        return Some(SpecId::ECOTONE);
    }

    Some(if header.timestamp >= 1710338135 {
        SpecId::CANCUN
    } else if header.timestamp >= 1681338455 {
        SpecId::SHANGHAI
    } else if header.total_difficulty?.saturating_sub(header.difficulty)
        >= U256::from(58_750_000_000_000_000_000_000_u128)
    {
        SpecId::MERGE
    } else if header.number? >= 12965000 {
        SpecId::LONDON
    } else if header.number? >= 12244000 {
        SpecId::BERLIN
    } else if header.number? >= 9069000 {
        SpecId::ISTANBUL
    } else if header.number? >= 7280000 {
        SpecId::PETERSBURG
    } else if header.number? >= 4370000 {
        SpecId::BYZANTIUM
    } else if header.number? >= 2675000 {
        SpecId::SPURIOUS_DRAGON
    } else if header.number? >= 2463000 {
        SpecId::TANGERINE
    } else if header.number? >= 1150000 {
        SpecId::HOMESTEAD
    } else {
        SpecId::FRONTIER
    })
}

/// Get the REVM block env of an Alloy block.
// https://github.com/paradigmxyz/reth/blob/280aaaedc4699c14a5b6e88f25d929fe22642fa3/crates/primitives/src/revm/env.rs#L23-L48
// TODO: Better error handling & properly test this, especially
// [blob_excess_gas_and_price].
pub(crate) fn get_block_env(header: &Header) -> Option<BlockEnv> {
    Some(BlockEnv {
        number: U256::from(header.number?),
        coinbase: header.miner,
        timestamp: U256::from(header.timestamp),
        gas_limit: U256::from(header.gas_limit),
        basefee: U256::from(header.base_fee_per_gas.unwrap_or_default()),
        difficulty: header.difficulty,
        prevrandao: header.mix_hash,
        blob_excess_gas_and_price: header
            .excess_blob_gas
            .map(|excess_blob_gas| BlobExcessGasAndPrice::new(excess_blob_gas as u64)),
    })
}

/// Represents errors that can occur when parsing transactions
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionParsingError {
    OverflowedGasLimit,
    MissingGasPrice,
    MissingMaxFeePerGas,
    InvalidType(u8),
    ConversionError(String),
}

/// Get the REVM tx envs of an Alloy block.
// https://github.com/paradigmxyz/reth/blob/280aaaedc4699c14a5b6e88f25d929fe22642fa3/crates/primitives/src/revm/env.rs#L234-L339
// https://github.com/paradigmxyz/reth/blob/280aaaedc4699c14a5b6e88f25d929fe22642fa3/crates/primitives/src/alloy_compat.rs#L112-L233
// TODO: Properly test this.
pub(crate) fn get_tx_env(tx: Transaction) -> Result<TxEnv, TransactionParsingError> {
    let enveloped_tx = {
        if tx.transaction_type.unwrap_or_default() == 126 {
            // TODO: implement this
            Bytes::new()
        } else {
            let tx_envelope = TxEnvelope::try_from(tx.clone())
                .map_err(|error| TransactionParsingError::ConversionError(error.to_string()))?;
            let mut envelope_buf = Vec::<u8>::new();
            tx_envelope.encode_2718(&mut envelope_buf);
            Bytes::from(envelope_buf)
        }
    };

    Ok(TxEnv {
        caller: tx.from,
        gas_limit: tx
            .gas
            .try_into()
            .map_err(|_| TransactionParsingError::OverflowedGasLimit)?,
        gas_price: match tx.transaction_type.unwrap() {
            0 | 1 => U256::from(
                tx.gas_price
                    .ok_or(TransactionParsingError::MissingGasPrice)?,
            ),
            2 | 3 => U256::from(
                tx.max_fee_per_gas
                    .ok_or(TransactionParsingError::MissingMaxFeePerGas)?,
            ),
            #[cfg(feature = "optimism")]
            // NOTE: Ideally, we should assert if chain is optimism.
            // However, the benefit (extra safety) does not justify the
            // inconvenience due to the modification of function signature.
            126 => U256::ZERO,
            unknown => return Err(TransactionParsingError::InvalidType(unknown)),
        },
        gas_priority_fee: tx.max_priority_fee_per_gas.map(U256::from),
        transact_to: match tx.to {
            Some(address) => TransactTo::Call(address),
            None => TransactTo::Create,
        },
        value: tx.value,
        data: tx.input,
        nonce: Some(tx.nonce),
        chain_id: tx.chain_id,
        access_list: tx
            .access_list
            .unwrap_or_default()
            .iter()
            .map(|access| {
                (
                    access.address,
                    access
                        .storage_keys
                        .iter()
                        .map(|&k| U256::from_be_bytes(*k))
                        .collect(),
                )
            })
            .collect(),
        blob_hashes: tx.blob_versioned_hashes.unwrap_or_default(),
        max_fee_per_blob_gas: tx.max_fee_per_blob_gas.map(U256::from),

        #[cfg(feature = "optimism")]
        optimism: OptimismFields {
            source_hash: tx
                .other
                .get_deserialized::<B256>("sourceHash")
                .map(|result| result.unwrap()),
            mint: tx
                .other
                .get_deserialized::<U128>("mint")
                .map(|result| result.unwrap().to()),
            is_system_transaction: tx
                .other
                .get_deserialized("isSystemTx")
                .map(|result| result.unwrap()),
            enveloped_tx: Some(enveloped_tx),
        },
    })
}
