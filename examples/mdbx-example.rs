//! This crate provides an example of using mdbx.

use anyhow::Result;
use clap::Parser;
use libmdbx::{Database, WriteFlags, WriteMap};

/// Command-line arguments.
#[derive(Parser, Debug)]
#[clap(name = "MDBX Example")]
struct Args {
    /// Sets the path to the database file
    #[clap(short, long, value_name = "FILE")]
    path: String,
}

/// This is the main function.
fn main() -> Result<()> {
    let args = Args::parse();

    let db = Database::<WriteMap>::open(&args.path)?;

    let tx = db.begin_rw_txn()?;
    let table = tx.open_table(None)?;
    tx.put(&table, "key", "value", WriteFlags::default())?;
    tx.commit()?;

    let tx = db.begin_ro_txn()?;
    let table = tx.open_table(None)?;
    let cursor = tx.cursor(&table)?;
    for item in cursor.into_iter() {
        println!("{:?}", item);
    }
    tx.commit()?;

    Ok(())
}
