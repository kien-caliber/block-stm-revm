//! Run PEVM against a mainnet block and verify

use alloy_chains::Chain;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{BlockId, BlockTransactionsKind};
use clap::Parser;
use pevm::{network::ethereum, RpcStorage, StorageWrapper};
use reqwest::Url;
use revm::db::CacheDB;
use std::error::Error;
use tokio::runtime::Runtime;

#[allow(missing_docs)]
#[path = "../tests/common/mod.rs"]
pub mod common;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// JSON RPC URL
    #[arg(long, env, default_value = "https://eth.llamarpc.com")]
    rpc_url: String,

    /// Block number
    #[arg()]
    block_number: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    // preprocess
    let args = Args::parse();
    let rpc_url = Url::parse(&args.rpc_url)?;
    let runtime = Runtime::new().unwrap();
    let provider = ProviderBuilder::new().on_http(rpc_url.clone());
    let block_maybe = runtime.block_on(provider.get_block(
        BlockId::number(args.block_number),
        BlockTransactionsKind::Full,
    ))?;
    let block = block_maybe.ok_or(Box::<dyn Error>::from("cannot fetch block"))?;
    let spec_id = ethereum::get_block_spec(&block.header).unwrap();
    let rpc_storage = RpcStorage::new(provider, spec_id, BlockId::number(args.block_number - 1));
    let wrapped_storage = StorageWrapper(&rpc_storage);
    let db = CacheDB::new(&wrapped_storage);
    common::test_execute_alloy(&db, Chain::mainnet(), block.clone(), true);

    Ok(())
}
