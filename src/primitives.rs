// TODO: Support custom chains like OP & RISE
// Ideally REVM & Alloy would provide all these.

use alloy_chains::Chain;
use alloy_consensus::TxEnvelope;
use alloy_primitives::{Bytes, TxKind, B256, U128};
use alloy_provider::network::eip2718::Encodable2718;
use alloy_rpc_types::{Header, Transaction};
use revm::primitives::{
    BlobExcessGasAndPrice, BlockEnv, OptimismFields, SpecId, TransactTo, TxEnv, U256,
};

use crate::EnvelopeBuilder;

/// Get the REVM spec id of an Alloy block.
// Currently hardcoding Ethereum hardforks from these reference:
// https://github.com/paradigmxyz/reth/blob/4fa627736681289ba899b38f1c7a97d9fcf33dc6/crates/primitives/src/revm/config.rs#L33-L78
// https://github.com/paradigmxyz/reth/blob/4fa627736681289ba899b38f1c7a97d9fcf33dc6/crates/primitives/src/chain/spec.rs#L44-L68
// TODO: Better error handling & properly test this.
pub fn get_block_spec(chain: Chain, header: &Header) -> Option<SpecId> {
    #[cfg(feature = "optimism")]
    if chain.is_optimism() {
        return Some(if header.timestamp >= 1710374401 {
            SpecId::ECOTONE
        } else if header.timestamp >= 1704992401 {
            SpecId::CANYON
        } else if header.timestamp >= 105235063 {
            SpecId::BEDROCK
        } else {
            SpecId::REGOLITH
        });
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

#[cfg(feature = "optimism")]
const DEPOSIT_TX_TYPE_ID: u8 = 126;

fn get_optimism_fields(tx: &Transaction) -> Result<OptimismFields, TransactionParsingError> {
    let source_hash = tx
        .other
        .get_deserialized::<B256>("sourceHash")
        .transpose()
        .map_err(|err| TransactionParsingError::ConversionError(err.to_string()))?;
    let mint: Option<u128> = match tx.other.get_deserialized::<U128>("mint").transpose() {
        Ok(opt) => opt.map(|value| value.to()),
        Err(err) => return Err(TransactionParsingError::ConversionError(err.to_string())),
    };
    let is_system_transaction = tx
        .other
        .get_deserialized("isSystemTx")
        .transpose()
        .map_err(|err| TransactionParsingError::ConversionError(err.to_string()))?;

    let envelope_buf = {
        if tx.transaction_type.unwrap_or_default() == DEPOSIT_TX_TYPE_ID {
            // https://github.com/paradigmxyz/reth/blob/3d3f52b2a4bf4fa0c8d94d44794a3f094cc76a5b/crates/primitives/src/transaction/optimism.rs#L110
            let mut eb = EnvelopeBuilder::with_capacity(8);
            eb.push(&source_hash.unwrap());
            eb.push(&tx.from);
            eb.push(&TxKind::from(tx.to));
            if let Some(mint) = &mint {
                eb.push(&mint);
            } else {
                eb.push(&[]);
            }
            eb.push(&tx.value);
            eb.push(&(tx.gas as u64));
            eb.push(&is_system_transaction.unwrap_or_default());
            eb.push(&tx.input);
            eb.to_bytes_with_header(Some(DEPOSIT_TX_TYPE_ID))
        } else {
            let tx_envelope = TxEnvelope::try_from(tx.clone())
                .map_err(|error| TransactionParsingError::ConversionError(error.to_string()))?;
            let mut envelope_buf = Vec::<u8>::new();
            tx_envelope.encode_2718(&mut envelope_buf);
            Bytes::from(envelope_buf)
        }
    };

    Ok(OptimismFields {
        source_hash,
        mint,
        is_system_transaction,
        enveloped_tx: Some(envelope_buf),
    })
}

/// Get the REVM tx envs of an Alloy block.
// https://github.com/paradigmxyz/reth/blob/280aaaedc4699c14a5b6e88f25d929fe22642fa3/crates/primitives/src/revm/env.rs#L234-L339
// https://github.com/paradigmxyz/reth/blob/280aaaedc4699c14a5b6e88f25d929fe22642fa3/crates/primitives/src/alloy_compat.rs#L112-L233
// TODO: Properly test this.
pub(crate) fn get_tx_env(tx: Transaction) -> Result<TxEnv, TransactionParsingError> {
    Ok(TxEnv {
        #[cfg(feature = "optimism")]
        optimism: get_optimism_fields(&tx)?,

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
    })
}
