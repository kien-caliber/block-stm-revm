//! Convert a block folder to a MDBX folder
//! Input folder contains block.json, block_hashes.json, bytecodes.json, pre_state.json.
//! Output folder contains mdbx.dat, mdbx.lck.

//! For help, run: `cargo run --example to-mdbx -- --help`

use alloy_primitives::{Bytes, B256};
use anyhow::Result;
use clap::Parser;
use libmdbx::{
    Database, DatabaseOptions, Mode, NoWriteMap, PageSize, ReadWriteOptions, SyncMode, TableFlags,
    WriteFlags,
};
use pevm::EvmCode;
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

fn create_all_tables(db: &Database<NoWriteMap>) -> Result<()> {
    let tx = db.begin_rw_txn()?;
    tx.create_table(Some("balance"), TableFlags::default())?;
    tx.create_table(Some("nonce"), TableFlags::default())?;
    tx.create_table(Some("code_hash"), TableFlags::default())?;
    tx.create_table(Some("bytecodes"), TableFlags::default())?;
    tx.create_table(Some("storage"), TableFlags::default())?;
    tx.commit()?;
    Ok(())
}

fn put_all(db: &Database<NoWriteMap>, table_name: &str, entries: &[(Bytes, Bytes)]) -> Result<()> {
    for (k, v) in entries {
        let tx = db.begin_rw_txn()?;
        let table = tx.open_table(Some(table_name))?;
        println!("{:?} {:?}", k, v.0.len());
        tx.put(&table, k, v, WriteFlags::UPSERT)?;
        tx.commit()?;
    }
    Ok(())
}

struct Data {
    bytecodes: HashMap<B256, EvmCode>,
}

impl Data {
    fn read_from(path: impl AsRef<Path>) -> Result<Self> {
        let bytecodes: HashMap<B256, EvmCode> = {
            let path = PathBuf::from(path.as_ref()).join("bytecodes.json");
            let file = File::open(path)?;
            let data = serde_json::from_reader(std::io::BufReader::new(file))?;
            data
        };

        Ok(Self { bytecodes })
    }

    fn write_to(&self, db: &Database<NoWriteMap>) -> Result<()> {
        create_all_tables(db)?;
        let bytecodes_entries: Vec<(Bytes, Bytes)> = self
            .bytecodes
            .iter()
            .map(|(code_hash, evm_code)| {
                (
                    Bytes::from(code_hash.clone()),
                    Bytecode::from(evm_code.clone()).bytes(),
                )
            })
            .collect();
        put_all(db, "bytecodes", &bytecodes_entries)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let db = Database::<NoWriteMap>::open_with_options(
        &args.output_dir,
        DatabaseOptions {
            max_tables: Some(16),
            mode: Mode::ReadWrite(ReadWriteOptions {
                sync_mode: SyncMode::Durable,
                min_size: Some(12288),
                max_size: Some(1073741824),
                growth_step: Some(8388608),
                shrink_threshold: Some(16777216),
            }),
            page_size: Some(PageSize::Set(4096)),
            ..DatabaseOptions::default()
        },
    )?;

    let data = Data::read_from(&args.input_dir)?;
    data.write_to(&db)?;

    Ok(())
}
