use std::path::Path;

use alloy_primitives::{keccak256, Address, Bytes, FixedBytes, B256, B64, U256};
use libmdbx::{DatabaseKind, DatabaseOptions, Mode, PageSize, ReadWriteOptions, SyncMode};
use revm::primitives::Bytecode;

use super::{AccountBasic, EvmAccount, EvmCode, Storage};

type B416 = FixedBytes<52>;

/// Represents an on-disk storage.
#[derive(Debug)]
pub struct OnDiskStorage {
    inner: libmdbx::Database<libmdbx::NoWriteMap>,
}

impl OnDiskStorage {
    /// Opens the on-disk storage at the specified path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, libmdbx::Error> {
        let db = libmdbx::Database::open_with_options(
            &path,
            DatabaseOptions {
                max_tables: Some(16),
                ..DatabaseOptions::default()
            },
        )?;
        Ok(Self { inner: db })
    }
}

fn get_fixed_bytes<E: DatabaseKind, const N: usize>(
    db: &libmdbx::Database<E>,
    table_name: &str,
    key: impl AsRef<[u8]>,
) -> Result<Option<FixedBytes<N>>, libmdbx::Error> {
    let tx = db.begin_ro_txn()?;
    let table = tx.open_table(Some(table_name))?;
    let bytes: Option<[u8; N]> = tx.get(&table, key.as_ref())?;
    if let Some(bytes) = bytes {
        Ok(Some(FixedBytes::from(bytes)))
    } else {
        Ok(None)
    }
}

fn get_bytes<E: DatabaseKind>(
    db: &libmdbx::Database<E>,
    table_name: &str,
    key: impl AsRef<[u8]>,
) -> Result<Option<Bytes>, libmdbx::Error> {
    let tx = db.begin_ro_txn()?;
    let table = tx.open_table(Some(table_name))?;
    let bytes: Option<Vec<u8>> = tx.get(&table, key.as_ref())?;
    if let Some(bytes) = bytes {
        Ok(Some(Bytes::copy_from_slice(bytes.as_slice())))
    } else {
        Ok(None)
    }
}

impl Storage for OnDiskStorage {
    type Error = libmdbx::Error;

    fn basic(&self, address: &Address) -> Result<Option<AccountBasic>, Self::Error> {
        let balance: Option<B256> = get_fixed_bytes(&self.inner, "balance", address)?;
        let nonce: Option<B64> = get_fixed_bytes(&self.inner, "nonce", address)?;
        match (balance, nonce) {
            (Some(balance), Some(nonce)) => Ok(Some(AccountBasic {
                balance: balance.into(),
                nonce: nonce.into(),
            })),
            _ => Ok(None),
        }
    }

    fn code_hash(&self, address: &Address) -> Result<Option<B256>, Self::Error> {
        get_fixed_bytes(&self.inner, "code_hash", address)
    }

    fn code_by_hash(&self, code_hash: &B256) -> Result<Option<EvmCode>, Self::Error> {
        let bytes = get_bytes(&self.inner, "bytecode", code_hash)?;
        let Some(bytes) = bytes else { return Ok(None) };
        Ok(Some(EvmCode::from(Bytecode::new_raw(bytes))))
    }

    fn has_storage(&self, address: &Address) -> Result<bool, Self::Error> {
        todo!()
    }

    fn storage(&self, address: &Address, index: &U256) -> Result<U256, Self::Error> {
        let composite_key =
            B416::from_slice(&[address.as_slice(), B256::from(*index).as_slice()].concat());
        let value = get_fixed_bytes(&self.inner, "storage", composite_key)?;
        Ok(value.map_or(U256::ZERO, From::from))
    }

    fn block_hash(&self, number: &u64) -> Result<B256, Self::Error> {
        Ok(keccak256(number.to_string().as_bytes())) // TODO:
    }
}
