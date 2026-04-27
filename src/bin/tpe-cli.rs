use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use toy_payments_engine::PaymentEngine;

/// Toy Payments Engine CLI
///
/// Reads transactions from a CSV file and processes them, then outputs the state of clients accounts as a CSV.
#[derive(Parser, Debug)]
struct Args {
    /// Path to the input CSV file containing transactions
    input: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::init();

    let mut engine = PaymentEngine::new();

    engine.process_transactions_from_file(&args.input)?;

    let accounts = engine.serialize_accounts()?;
    println!("{}", accounts);

    Ok(())
}
