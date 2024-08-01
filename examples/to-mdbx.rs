//! Convert a block folder to a MDBX folder
//! Input folder contains block.json, block_hashes.json, bytecodes.json, pre_state.json.
//! Output folder contains mdbx.dat, mdbx.lck.

//! For help, run: `cargo run --example to-mdbx -- --help`

use alloy_primitives::{Address, Bytes, B256, B64, U256};
use anyhow::Result;
use clap::Parser;
use libmdbx::{
    Database, DatabaseKind, DatabaseOptions, Mode, NoWriteMap, ReadWriteOptions, SyncMode,
    TableFlags, WriteFlags,
};
use pevm::{EvmAccount, EvmCode};
use revm::primitives::Bytecode;
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Parser, Debug)]
#[clap(name = "to-mdbx")]
struct Args {
    #[clap(short, long, value_name = "DIRECTORY")]
    input_dir: String,
    #[clap(short, long, value_name = "DIRECTORY")]
    output_dir: String,
}

const MB: isize = 1048576;

fn open_db(dir: impl AsRef<Path>) -> Result<Database<NoWriteMap>> {
    let db = Database::<NoWriteMap>::open_with_options(
        dir.as_ref(),
        DatabaseOptions {
            max_tables: Some(16),
            mode: Mode::ReadWrite(ReadWriteOptions {
                // https://erthink.github.io/libmdbx/group__c__settings.html#ga79065e4f3c5fb2ad37a52b59224d583e
                // https://github.com/erthink/libmdbx/issues/136#issuecomment-727490550
                sync_mode: SyncMode::Durable,
                min_size: Some(1 * MB), // The lower bound allows you to prevent database shrinking below certain reasonable size to avoid unnecessary resizing costs.
                max_size: Some(1024 * MB), // The upper bound allows you to prevent database growth above certain reasonable size.
                growth_step: Some(1 * MB), // The growth step must be greater than zero to allow the database to grow, but also reasonable not too small, since increasing the size by little steps will result a large overhead.
                shrink_threshold: Some(4 * MB), // The shrink threshold must be greater than zero to allow the database to shrink but also reasonable not too small (to avoid extra overhead) and not less than growth step to avoid up-and-down flouncing.
            }),
            ..DatabaseOptions::default()
        },
    )?;

    Ok(db)
}

fn create_all_tables(db: &Database<NoWriteMap>) -> Result<()> {
    let tx = db.begin_rw_txn()?;
    tx.create_table(Some("balance"), TableFlags::default())?;
    tx.create_table(Some("nonce"), TableFlags::default())?;
    tx.create_table(Some("code_hash"), TableFlags::default())?;
    tx.create_table(Some("bytecode"), TableFlags::default())?;
    tx.create_table(Some("storage"), TableFlags::default())?;
    tx.commit()?;
    Ok(())
}

fn put_all<E, K, V>(
    db: &Database<E>,
    table_name: &str,
    entries: impl Iterator<Item = (K, V)>,
) -> Result<()>
where
    E: DatabaseKind,
    K: AsRef<[u8]>,
    V: AsRef<[u8]>,
{
    let tx = db.begin_rw_txn()?;
    let table = tx.open_table(Some(table_name))?;
    for (k, v) in entries {
        tx.put(&table, k, v, WriteFlags::UPSERT)?;
    }
    tx.commit()?;
    Ok(())
}

struct Data {
    balances: HashMap<Address, B256>,
    nonces: HashMap<Address, B64>,
    code_hashes: HashMap<Address, B256>,
    codes: HashMap<B256, Bytes>,
    storage: HashMap<(Address, U256), B256>,
}

impl Data {
    fn read_from(path: impl AsRef<Path>) -> Result<Self> {
        let state: HashMap<Address, EvmAccount> = {
            let path = PathBuf::from(path.as_ref()).join("pre_state.json");
            let file = File::open(path)?;
            serde_json::from_reader(std::io::BufReader::new(file))?
        };

        let balances: HashMap<Address, B256> = state
            .iter()
            .map(|(&address, account)| (address, B256::from(account.basic.balance)))
            .collect();

        let nonces: HashMap<Address, B64> = state
            .iter()
            .map(|(&address, account)| (address, B64::from(account.basic.nonce)))
            .collect();

        let code_hashes: HashMap<Address, B256> = state
            .iter()
            .filter_map(|(&address, account)| {
                if let Some(code_hash) = account.code_hash {
                    Some((address, code_hash))
                } else {
                    None
                }
            })
            .collect();

        let mut storage: HashMap<(Address, U256), B256> = HashMap::new();
        for (address, account) in state {
            for (key, value) in account.storage {
                storage.insert((address, key), B256::from(value));
            }
        }

        let bytecodes: HashMap<B256, EvmCode> = {
            let path = PathBuf::from(path.as_ref()).join("bytecodes.json");
            let file = File::open(path)?;
            serde_json::from_reader(std::io::BufReader::new(file))?
        };

        let codes: HashMap<B256, Bytes> = bytecodes
            .into_iter()
            .map(|(code_hash, evm_code)| (code_hash, Bytecode::from(evm_code).bytes()))
            .collect();

        Ok(Self {
            balances,
            nonces,
            code_hashes,
            codes,
            storage,
        })
    }

    fn write_to(&self, db: &Database<NoWriteMap>) -> Result<()> {
        create_all_tables(db)?;
        put_all(db, "bytecode", self.codes.iter())?;
        put_all(db, "balance", self.balances.iter())?;
        put_all(db, "nonce", self.nonces.iter())?;
        put_all(db, "code_hash", self.code_hashes.iter())?;
        put_all(
            db,
            "storage",
            self.storage.iter().map(|(&(address, key), value)| {
                let composite_key = Bytes::copy_from_slice(
                    &[address.as_slice(), B256::from(key).as_slice()].concat(),
                );
                (composite_key, value)
            }),
        )?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let db = open_db(args.output_dir)?;
    let data = Data::read_from(&args.input_dir)?;
    data.write_to(&db)?;

    Ok(())
}
