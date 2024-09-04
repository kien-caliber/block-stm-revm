//! Fetch optimism blocks and write to files
#![cfg(feature = "optimism")]

use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File},
    io::BufReader,
    path::Path,
};

use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_primitives::{Address, B256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{Block, BlockId, BlockTransactionsKind};
use clap::Parser;
use op_alloy_network::Optimism;
use op_alloy_rpc_types::Transaction;
use pevm::{
    chain::{PevmChain, PevmOptimism},
    EvmAccount, EvmCode, Pevm, RpcStorage,
};
use reqwest::Url;
use tokio::runtime::Runtime;

#[derive(Parser, Debug)]
struct Args {
    /// The block number to query
    block_number: u64,

    /// The RPC URL
    #[arg(long, default_value = "https://mainnet.optimism.io")]
    rpc_url: String,

    #[arg(long)]
    write_block_to: Option<String>,

    #[arg(long)]
    write_pre_state_to: Option<String>,

    #[arg(long)]
    write_bytecodes_to: Option<String>,
}

type FetchedBlock = (
    Block<Transaction>,
    BTreeMap<Address, EvmAccount>,
    BTreeMap<B256, EvmCode>,
);

fn fetch_block(
    block_number: u64,
    rpc_url: Url,
) -> Result<FetchedBlock, Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    let provider = ProviderBuilder::<_, _, Optimism>::default().on_http(rpc_url);
    let block = runtime
        .block_on(provider.get_block(BlockId::number(block_number), BlockTransactionsKind::Full))?
        .ok_or("missing block")?;

    let chain = PevmOptimism::mainnet();
    let spec_id = chain.get_block_spec(&block.header).unwrap();
    let pre_state_rpc_storage =
        RpcStorage::new(provider.clone(), spec_id, BlockId::number(block_number - 1));
    let concurrency_level =
        std::thread::available_parallelism().unwrap_or(std::num::NonZeroUsize::MIN);

    let mut pevm = Pevm::default();
    pevm.execute(
        &pre_state_rpc_storage,
        &chain,
        block.clone(),
        concurrency_level,
        true,
    )
    .unwrap();

    let mut state = BTreeMap::<Address, EvmAccount>::new();
    let mut bytecodes: BTreeMap<B256, EvmCode> =
        BTreeMap::from_iter(pre_state_rpc_storage.get_cache_bytecodes());
    for (address, mut account) in pre_state_rpc_storage.get_cache_accounts() {
        if let Some(code) = account.code.take() {
            assert_ne!(account.code_hash.unwrap(), KECCAK_EMPTY);
            bytecodes.insert(account.code_hash.unwrap(), code);
        }
        state.insert(address, account);
    }

    Ok((block, state, bytecodes))
}

fn write_block(
    path: impl AsRef<Path>,
    block: Block<Transaction>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent)?;
    }
    let file = File::create(path.as_ref())?;
    serde_json::to_writer(file, &block)?;
    Ok(())
}

fn write_pre_state(
    path: impl AsRef<Path>,
    pre_state: BTreeMap<Address, EvmAccount>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent)?;
    }
    let file = File::create(path.as_ref())?;
    serde_json::to_writer(file, &pre_state)?;
    Ok(())
}

fn write_bytecodes(
    path: impl AsRef<Path>,
    bytecodes: BTreeMap<B256, EvmCode>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent)?;
    }
    let mut accumulated_bytecodes: BTreeMap<B256, EvmCode> = match File::open(path.as_ref()) {
        Ok(file) => bincode::deserialize_from(BufReader::new(file)).unwrap(),
        Err(_) => BTreeMap::new(),
    };
    accumulated_bytecodes.extend(bytecodes);
    let file = File::create(path.as_ref())?;
    bincode::serialize_into(file, &accumulated_bytecodes)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let rpc_url = Url::parse(&args.rpc_url)?;
    let (block, pre_state, bytecodes) = fetch_block(args.block_number, rpc_url)?;

    dbg!(&block.header.gas_used);
    dbg!(pre_state.len());
    dbg!(bytecodes.len());

    if let Some(path) = args.write_block_to {
        write_block(path, block)?;
    }

    if let Some(path) = args.write_pre_state_to {
        write_pre_state(path, pre_state)?;
    }

    if let Some(path) = args.write_bytecodes_to {
        write_bytecodes(path, bytecodes)?;
    }

    Ok(())
}
