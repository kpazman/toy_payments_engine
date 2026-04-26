use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use toy_payments_engine::Transaction;

#[derive(Parser, Debug)]
struct Args {
    /// Path to the input CSV file containing transactions
    input: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::init();

    log::debug!("Reading transactions from {}", args.input.display());

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(args.input)?;
    for result in reader.deserialize() {
        let transaction: Transaction = result?;
        log::debug!("Transaction: {:?}", transaction);
    }

    Ok(())
}
