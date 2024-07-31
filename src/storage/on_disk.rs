use std::path::Path;

use alloy_primitives::{Address, B256, U256};
use libmdbx::{DatabaseOptions, Mode, PageSize, ReadWriteOptions, SyncMode};

use super::{AccountBasic, EvmAccount, EvmCode, Storage};

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

impl Storage for OnDiskStorage {
    type Error = libmdbx::Error;

    fn basic(&self, address: &Address) -> Result<Option<AccountBasic>, Self::Error> {
        todo!()
    }

    fn code_hash(&self, address: &Address) -> Result<Option<B256>, Self::Error> {
        todo!()
    }

    fn code_by_hash(&self, code_hash: &B256) -> Result<Option<EvmCode>, Self::Error> {
        todo!()
    }

    fn has_storage(&self, address: &Address) -> Result<bool, Self::Error> {
        todo!()
    }

    fn storage(&self, address: &Address, index: &U256) -> Result<U256, Self::Error> {
        todo!()
    }

    fn block_hash(&self, number: &u64) -> Result<B256, Self::Error> {
        todo!()
    }
}

pub fn convert_to_mbdx(
    path: impl AsRef<Path>,
    accounts: impl IntoIterator<Item = (Address, EvmAccount)>,
    bytecodes: impl IntoIterator<Item = (B256, EvmCode)>,
    block_hashes: impl IntoIterator<Item = (u64, B256)>,
) {
    todo!();
}
