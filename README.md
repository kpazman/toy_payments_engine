# Toy Payment Engine

A simple toy payments engine that reads a series of transactions from a CSV, updates client accounts, handles disputes and chargebacks, and then outputs the state of clients accounts as a CSV.

## Usage

The input file is the first and only argument to the binary. Output is written to std out.

```bash
cargo run -- transactions.csv > accounts.csv
```

## AI disclaimer

The project was developed in Cursor, tab completions might have been accepted then modified as needed, but no coding agents were used.
