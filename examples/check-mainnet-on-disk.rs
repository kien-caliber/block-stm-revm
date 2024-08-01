//! Check mainnet blocks using [OnDiskStorage]

use std::{fs::File, io::BufReader};

use alloy_rpc_types::Block;
use anyhow::Result;
use clap::Parser;
use pevm::{chain::PevmEthereum, OnDiskStorage, StorageWrapper};
use revm::db::CacheDB;

#[path = "../tests/common/mod.rs"]
mod common;

#[derive(Parser, Debug)]
#[clap(name = "check-mainnet-on-disk")]
struct Args {
    #[clap(short, long, value_name = "DIRECTORY")]
    mdbx_dir: String,
    #[clap(short, long, value_name = "PATH")]
    block_path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let on_disk_storage = OnDiskStorage::open(args.mdbx_dir)?;
    let wrapped_storage = StorageWrapper(&on_disk_storage);
    let db = CacheDB::new(&wrapped_storage);
    let chain = PevmEthereum::mainnet();
    let block: Block =
        serde_json::from_reader(BufReader::new(File::open(args.block_path).unwrap())).unwrap();
    common::test_execute_alloy(&db, &chain, block, true);
    Ok(())
}
