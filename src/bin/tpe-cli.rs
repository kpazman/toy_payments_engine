use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    /// Path to the input CSV file containing transactions
    input: PathBuf,
}

fn main() {
    let args = Args::parse();
    println!("Reading transactions from {}", args.input.display());
}
